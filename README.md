# Rusty Games

Experimental codebase for games written in rust.

## Setup

### Vulkan

We use Vulkan as our 3D renderer. You must have Vulkan 1.3 installed:

- https://vulkan.lunarg.com/sdk/home

### GLFW

[GLFW](https://www.glfw.org/) is used for windowing support. In order for this to work it needs to interact with the native OS, which means we need to link against the correct version of GLFW. In order to create reproducable builds we include the `.lib` files for GLFW in source control and switch which to use based on [cargo configuration files](https://doc.rust-lang.org/cargo/reference/config.html#targettriplelinks).

Inside `.cargo` is a `config.toml` file where we specify platform specific build instructions, including override the `build.rs` file for `glfw`. If your platform fails to build `glfw` by default, download (or compile) glfw to product the appropriate `.lib` file and add it in the `deps` folder, following the existing folders as examples. You can run `rustc --print cfg` and `rustc --print targets-list` to find the appropriate triple to add. Finally, add an entry to `.cargo/config.toml` for your tiple and path.

### Troubleshooting

#### note: LINK : fatal error LNK1181: cannot open input file 'vulkan-1.lib'

Ensure you have Vulkan installed and the file location included in your build path environment variables. On windows this means editing your PATH environment variable to include the Vulkan install location (typically something like `C:\VulkanSDK\<version>\Lib`)
