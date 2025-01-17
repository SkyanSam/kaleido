//! Illustrates rendering using Vulkan with multiview. Supports any Vulkan 1.1 capable environment.
//!
//! Renders a smooth gradient across the entire view, with different colors per eye.
//!
//! This example uses minimal abstraction for clarity. Real-world code should encapsulate and
//! largely decouple its Vulkan and OpenXR components and handle errors gracefully.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use ash::{
    vk::{self, Handle},
};
use openxr as xr;

mod kabstract;
use kabstract::*;

mod kstructs;
use kstructs::*;

mod kconstants;
use kconstants::*;

#[allow(clippy::field_reassign_with_default)] // False positive, might be fixed 1.51
#[cfg_attr(target_os = "android", ndk_glue::main)]
#[allow(clippy::field_reassign_with_default)]
#[cfg_attr(target_os = "android", ndk_glue::main)]
pub fn main() {

    // Handle interrupts gracefully
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    })
        .expect("setting Ctrl-C handler");

    let (xr_instance, system, environment_blend_mode) = init_openxr();

    let vk_target_version = vk::make_api_version(0, 1, 1, 0);
    let vk_target_version_xr = xr::Version::new(1, 1, 0);
    let reqs = xr_instance
        .graphics_requirements::<xr::Vulkan>(system)
        .unwrap();

    if vk_target_version_xr < reqs.min_api_version_supported
        || vk_target_version_xr.major() > reqs.max_api_version_supported.major()
    {
        panic!(
            "OpenXR runtime requires Vulkan version > {}, < {}.0.0",
            reqs.min_api_version_supported,
            reqs.max_api_version_supported.major() + 1
        );
    }

    let (vk_instance, vk_physical_device, vk_device, queue, queue_family_index) =
        init_vulkan(&xr_instance, system, vk_target_version);

    let render_pass = create_render_pass(&vk_device);
    let (pipeline, pipeline_layout) = create_pipeline(&vk_device, render_pass);

    let (session, mut frame_wait, mut frame_stream) = unsafe {
        xr_instance.create_session::<xr::Vulkan>(
            system,
            &xr::vulkan::SessionCreateInfo {
                instance: vk_instance.handle().as_raw() as _,
                physical_device: vk_physical_device.as_raw() as _,
                device: vk_device.handle().as_raw() as _,
                queue_family_index,
                queue_index: 0,
            }
        )
    }.unwrap();

    let (stage, action_set, left_action, right_action, left_space, right_space) = setup_openxr(&xr_instance, system, &session);

    let (cmd_pool, cmds, fences) = create_commands(&vk_device, queue_family_index);

    // Main loop
    let mut swapchain = None;
    let mut event_storage = xr::EventDataBuffer::new();
    let mut session_running = false;
    let mut frame = 0;
    'main_loop: loop {
        if !running.load(Ordering::Relaxed) {
            println!("requesting exit");
            match session.request_exit() {
                Ok(()) => {}
                Err(xr::sys::Result::ERROR_SESSION_NOT_RUNNING) => break,
                Err(e) => panic!("{}", e),
            }
        }

        while let Some(event) = xr_instance.poll_event(&mut event_storage).unwrap() {
            use xr::Event::*;
            match event {
                SessionStateChanged(e) => {
                    // Session state change is where we can begin and end sessions, as well as
                    // find quit messages!
                    println!("entered state {:?}", e.state());
                    match e.state() {
                        xr::SessionState::READY => {
                            session.begin(VIEW_TYPE).unwrap();
                            session_running = true;
                        }
                        xr::SessionState::STOPPING => {
                            session.end().unwrap();
                            session_running = false;
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            break 'main_loop;
                        }
                        _ => {}
                    }
                }
                InstanceLossPending(_) => {
                    break 'main_loop;
                }
                EventsLost(e) => {
                    println!("lost {} events", e.lost_event_count());
                }
                _ => {}
            }
        }

        if !session_running {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }

        let xr_frame_state = frame_wait.wait().unwrap();
        frame_stream.begin().unwrap();

        if !xr_frame_state.should_render {
            frame_stream.end(xr_frame_state.predicted_display_time, environment_blend_mode, &[]).unwrap();
            continue;
        }

        let swapchain = create_swapchain(&mut swapchain, &xr_instance, &vk_device, render_pass, system, &session);

        let image_index = swapchain.handle.acquire_image().unwrap();

            unsafe {
                vk_device.wait_for_fences(&[fences[frame]], true, u64::MAX).unwrap();
                vk_device.reset_fences(&[fences[frame]]).unwrap();
            }

        let cmd = cmds[frame];
        unsafe {
            vk_device.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)).unwrap();
            vk_device.cmd_begin_render_pass(
                cmd,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(render_pass)
                    .framebuffer(swapchain.buffers[image_index as usize].framebuffer)
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D::default(),
                        extent: swapchain.resolution,
                    })
                    .clear_values(&[vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    }]),
                vk::SubpassContents::INLINE,
            );
        }

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: swapchain.resolution.width as f32,
            height: swapchain.resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain.resolution,
        }];

        unsafe {
            vk_device.cmd_set_viewport(cmd, 0, &viewports);
            vk_device.cmd_set_scissor(cmd, 0, &scissors);

            vk_device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
            vk_device.cmd_draw(cmd, 3, 1, 0, 0);

            vk_device.cmd_end_render_pass(cmd);
            vk_device.end_command_buffer(cmd).unwrap();
        }

        session.sync_actions(&[(&action_set).into()]).unwrap();

        let right_location = right_space.locate(&stage, xr_frame_state.predicted_display_time).unwrap();
        let left_location = left_space.locate(&stage, xr_frame_state.predicted_display_time).unwrap();

        let mut printed = false;
        if left_action.is_active(&session, xr::Path::NULL).unwrap() {
            print!(
                "Left Hand: ({:0<12},{:0<12},{:0<12}), ",
                left_location.pose.position.x,
                left_location.pose.position.y,
                left_location.pose.position.z
            );
            printed = true;
        }

        if right_action.is_active(&session, xr::Path::NULL).unwrap() {
            print!(
                "Right Hand: ({:0<12},{:0<12},{:0<12})",
                right_location.pose.position.x,
                right_location.pose.position.y,
                right_location.pose.position.z
            );
            printed = true;
        }
        if printed {
            println!();
        }

        let (_, views) = session.locate_views(VIEW_TYPE, xr_frame_state.predicted_display_time, &stage).unwrap();
        swapchain.handle.wait_image(xr::Duration::INFINITE).unwrap();

        unsafe {
            vk_device.queue_submit(queue, &[vk::SubmitInfo::default().command_buffers(&[cmd])], fences[frame]).unwrap();
        }
        swapchain.handle.release_image().unwrap();

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: swapchain.resolution.width as _,
                height: swapchain.resolution.height as _,
            },
        };

        frame_stream.end(xr_frame_state.predicted_display_time, environment_blend_mode, &[&xr::CompositionLayerProjection::new().space(&stage).views(&[xr::CompositionLayerProjectionView::new().pose(views[0].pose).fov(views[0].fov).sub_image(xr::SwapchainSubImage::new().swapchain(&swapchain.handle).image_array_index(0).image_rect(rect)), xr::CompositionLayerProjectionView::new().pose(views[1].pose).fov(views[1].fov).sub_image(xr::SwapchainSubImage::new().swapchain(&swapchain.handle).image_array_index(1).image_rect(rect))])]).unwrap();
        frame = (frame + 1) % PIPELINE_DEPTH as usize;
    }

    unsafe {
        drop((session, frame_wait, frame_stream, stage, action_set, left_space, right_space, left_action, right_action));
        vk_device.wait_for_fences(&fences, true, !0).unwrap();
        for fence in fences {
            vk_device.destroy_fence(fence, None);
        }

        if let Some(swapchain) = swapchain {
            for buffer in swapchain.buffers {
                vk_device.destroy_framebuffer(buffer.framebuffer, None);
                vk_device.destroy_image_view(buffer.color, None);
            }
        }

        vk_device.destroy_pipeline(pipeline, None);
        vk_device.destroy_pipeline_layout(pipeline_layout, None);
        vk_device.destroy_command_pool(cmd_pool, None);
        vk_device.destroy_render_pass(render_pass, None);
        vk_device.destroy_device(None);
        vk_instance.destroy_instance(None);
    }

    println!("exiting cleanly");
}

