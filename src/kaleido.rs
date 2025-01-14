use std::{borrow::Cow, cell::RefCell, default::Default, error::Error, ffi, ops::Drop, os::raw::c_char, ptr};
use std::ffi::{c_int, CString};
use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk,
    vk::Handle,
    Device, Entry, Instance,
};
use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::sys::{SDL_CreateWindow, SDL_Init, SDL_RenderClear, SDL_RenderGeometry, SDL_RenderPresent, SDL_SetRenderDrawColor, SDL_Vulkan_LoadLibrary, SDL_Window, SDL_ALPHA_OPAQUE, SDL_INIT_AUDIO, SDL_INIT_VIDEO, SDL_WINDOWPOS_UNDEFINED_MASK};
use sdl2::sys::SDL_WindowFlags::{SDL_WINDOW_SHOWN, SDL_WINDOW_VULKAN};
use sdl2::video::Window;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Window Title - My Vulkano-SDL2 application", 1024, 768)
        .vulkan()
        .build()
        .unwrap();
    let instance_extensions = window.vulkan_instance_extensions().unwrap();


    let entry = unsafe { Entry::load().unwrap() };
    let instance = create_vulkan_instance(&entry, &window);
    let surface = create_surface(&entry, &instance, &window);


    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                _ => {}
            }
        }
        ::std::thread::sleep(::std::time::Duration::new(0, 1_000_000_000u32 / 60));
        /*unsafe {
            SDL_SetRenderDrawColor(renderer, 0, 0, 0, SDL_ALPHA_OPAQUE);
            SDL_RenderClear(renderer);
            SDL_RenderGeometry(renderer, nullptr, verts.data(), verts.size(), nullptr, 0);
            SDL_RenderPresent(renderer );
        }*/
    }

    unsafe {
        // may need to destroy surface here
        instance.destroy_instance(None);
    }

    /*
    unsafe {
        let c_string = CString::new("Hello Window").expect("");
        let c_str_ptr: *const i8 = c_string.as_ptr();
        SDL_Init(SDL_INIT_VIDEO | SDL_INIT_AUDIO);
        SDL_Vulkan_LoadLibrary(ptr::null());
        // needs SDL_Window_Vulkan
        let window = SDL_CreateWindow(c_str_ptr, SDL_WINDOWPOS_UNDEFINED_MASK as c_int, SDL_WINDOWPOS_UNDEFINED_MASK as c_int, 640, 360, SDL_WINDOW_SHOWN | SDL_WINDOW_VULKAN);
        SDL_Vulkan_GetInstanceExtensions(window, )

    }*/
}

fn create_vulkan_instance(entry: &Entry, window: &Window) -> ash::Instance {
    let app_name = std::ffi::CString::new("Vulkan App").unwrap();

    let app_info = vk::ApplicationInfo::default()
        .application_name(&app_name)
        .application_version(0)
        .engine_name(&app_name)
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 3, 0)); // Vulkan 1.3

    // Get required Vulkan extensions from SDL2
    let required_extensions = window
        .vulkan_instance_extensions()
        .expect("Failed to get Vulkan instance extensions");

    let extension_pointers: Vec<*const i8> = required_extensions
        .iter()
        .map(|ext| ext.as_ptr() as *const i8)
        .collect();

    let instance_create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extension_pointers);

    unsafe {
        entry
            .create_instance(&instance_create_info, None)
            .expect("Failed to create Vulkan instance")
    }
}
fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &Window,
) -> vk::SurfaceKHR {
    let surface = window
        .vulkan_create_surface(instance.handle().as_raw() as _)
        .expect("Failed to create Vulkan surface");

    vk::SurfaceKHR::from_raw(surface as _)
}