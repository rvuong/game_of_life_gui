// Only sim is part of the library so integration tests in tests/ can reach it
// without pulling in wgpu/winit as test dependencies.
pub mod sim;
