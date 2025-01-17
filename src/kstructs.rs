
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

pub(crate) struct Swapchain {
    pub handle: xr::Swapchain<xr::Vulkan>,
    pub buffers: Vec<crate::Framebuffer>,
    pub resolution: vk::Extent2D,
}

pub(crate) struct Framebuffer {
    pub framebuffer: vk::Framebuffer,
    pub color: vk::ImageView,
}
