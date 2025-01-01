// this is where we write the new code
use std::{
    io::Cursor,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use ash::{
    util::read_spv,
    vk::{self, Handle},
};
use openxr as xr;

use std::iter;

use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

fn initialize_wgpu_openxr(
    xr_instance: &openxr::Instance,
    xr_system: openxr::SystemId,
) -> (
    ash::Entry,
    ash::Instance,
    ash::vk::PhysicalDevice,
    u32,
    ash::Device,
    wgpu::Instance,
    wgpu::Adapter,
    wgpu::Device,
    wgpu::Queue,
) {
    unsafe {
        // This must always be called before vulkan init
        let _requirements = xr_instance
            .graphics_requirements::<openxr::Vulkan>(xr_system)
            .unwrap();

        // Initialize Vulkan instance (!! THIS IS LOAD AT THE MOMENT, NOT LINKED OR STATIC)
        let vk_entry = ash::Entry::load().unwrap();

        let vk_extensions = vk_entry.

        //wgpu::Instance::required_vulkan_extensions(&vk_entry);
        let mut extension_names_raw = vec![];
        for extension in &vk_extensions {
            extension_names_raw.push(extension.as_ptr());
        }
        let vk_target_version = vk::make_version(1, 1, 0);
        let vk_app_info = vk::ApplicationInfo {
            s_type: Default::default(),
            p_next: (),
            p_application_name: (),
            application_version: 0,
            p_engine_name: (),
            engine_version: 0,
            api_version: vk_target_version,
            _marker: Default::default(),
        };
        let instance_create_info = vk::InstanceCreateInfo {
            s_type: Default::default(),
            p_next: (),
            flags: Default::default(),
            p_application_info: &vk_app_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: (),
            enabled_extension_count: 0,
            pp_enabled_extension_names: &extension_names_raw,
            _marker: Default::default(),
        };

        let vk_instance = xr_instance
            .create_vulkan_instance(
                xr_system,
                std::mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
                &instance_create_info as *const _
                    as *const _,
            )
            .expect("XR error creating Vulkan instance")
            .map_err(vk::Result::from_raw)
            .expect("Vulkan error creating Vulkan instance");
        let vk_instance = ash::Instance::load(
            vk_entry.static_fn(),
            vk::Instance::from_raw(vk_instance as _),
        );

        // Find the physical device we actually need to initialize with
        let vk_physical_device = vk::PhysicalDevice::from_raw(
            xr_instance
                .vulkan_graphics_device(xr_system, vk_instance.handle().as_raw() as _)
                .unwrap() as _,
        );

        let queue_family_index = vk_instance
            .get_physical_device_queue_family_properties(vk_physical_device)
            .into_iter()
            .enumerate()
            .find_map(|(queue_family_index, info)| {
                if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    Some(queue_family_index as u32)
                } else {
                    None
                }
            })
            .expect("Vulkan device has no graphics queue");

        // Initialize WGPU instance using our Vulkan instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: Default::default(),
            dx12_shader_compiler: Default::default(),
            gles_minor_version: Default::default(),
        });
        // IDK ABOUT THIS NEW RAW VULKAN!!!!!
        let instance =
            wgpu::Instance::new_raw_vulkan(vk_entry.clone(), vk_instance.clone(), vk_extensions);
        let adapter = instance.adapter_from_raw_vulkan(vk_physical_device);

        // Create the Vulkan logical device
        let desc = wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
        };
        let vk_device_extensions = adapter.required_vulkan_device_extensions(&desc);
        let mut device_extension_names_raw = vec![];
        for extension in &vk_device_extensions {
            device_extension_names_raw.push(extension.as_ptr());
        }

        let queue_create_infos = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])
            .build()];
        let mut vulkan11_features = vk::PhysicalDeviceVulkan11Features {
            multiview: vk::TRUE,
            ..Default::default()
        };
        let create_device_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extension_names_raw)
            .push_next(&mut vulkan11_features);
        let vk_device_raw = xr_instance
            .create_vulkan_device(
                xr_system,
                std::mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
                vk_physical_device.as_raw() as _,
                &create_device_info as *const _ as *const _,
            )
            .expect("XR error creating Vulkan device")
            .map_err(vk::Result::from_raw)
            .expect("Vulkan error creating Vulkan device");
        let vk_device = ash::Device::load(
            vk_instance.fp_v1_0(),
            vk::Device::from_raw(vk_device_raw as _),
        );

        // Initialize WGPU device using our Device instance
        let (device, queue) =
            adapter.device_from_raw_vulkan(vk_device.clone(), queue_family_index, &desc, None);

        (
            vk_entry,
            vk_instance,
            vk_physical_device,
            queue_family_index,
            vk_device,
            instance,
            adapter,
            device,
            queue,
        )
    }
}

fn main() {

}