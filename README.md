# Rusty Games

Experimental codebase for games written in rust.

## Setup

### Vulkan

We use Vulkan as our 3D renderer. You must have Vulkan 1.3 installed:

- https://vulkan.lunarg.com/sdk/home

### Troubleshooting

#### note: LINK : fatal error LNK1181: cannot open input file 'vulkan-1.lib'

Ensure you have Vulkan installed and the file location included in your build path environment variables. On windows this means editing your PATH environment variable to include the Vulkan install location (typically something like `C:\VulkanSDK\<version>\Lib`)
