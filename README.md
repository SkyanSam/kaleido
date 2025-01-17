This project features two packages.

vulkan which is the exact code from openxrs/openxr/examples.

kvulkan which is the same code but abstracted away.
These abstractions can be found in kabstract, kconstants, and kstructs.

To run:
cargo run --example kvulkan --features static
