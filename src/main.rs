mod renderer;
mod camera;
mod model;
mod scene;

use winit::{
    event::*,
    event_loop::{EventLoop, ControlFlow},
    window::{WindowBuilder}
};

use std::path::Path;
use std::time::{Instant};

use renderer::{
    Renderer,
};

use scene::{
    Scene,
    DrawScene,
};

use model::{
    Model,
};

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new() .build(&event_loop) .unwrap();

    let _res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");

    // fn main() cannot be async, so block the main thread until future complete.
    use futures::executor::block_on;
    let mut renderer = block_on(Renderer::new(&window));

    // Create scene, add a model to it.
    let mut scene = Scene::empty();
    let res_dir = Path::new(env!("OUT_DIR")).join("res");
    let model = Model::load(&renderer, res_dir.join("avocado").join("Avocado.glb")).unwrap();
    scene.add_model(model);

    let mut last_render_time = Instant::now();
    let mut spawn_time = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            // TODO: Probably shouldn't do any input processing in renderer but move it to a seperate mod?
            Event::DeviceEvent { ref event, .. } => { renderer.input_mouse_movement(event); }
            Event::WindowEvent { ref event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        // TODO: Probably shouldn't do any input processing in renderer but move it to a seperate mod?
                        _ => { renderer.input(event); }
                    }
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(Some(*physical_size));
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        renderer.resize(Some(**new_inner_size));
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(_) => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;

                // Timer to test adding instances at runtime.
                if spawn_time.elapsed().as_secs_f32() > 3.0 {
                    scene.add_instance_of(0);
                    spawn_time = Instant::now();
                }

                renderer.update(dt);
                match renderer.draw_scene(&scene) {
                    // All good.
                    Ok(_) => {}
                    // Recreate the sc if it is lost.
                    Err(wgpu::SwapChainError::Lost) => renderer.resize(None),
                    // Out of mem, just exit the program.
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errrors should be resolved by the next frame.
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested is only triggered once, we need to do so manually in the loop:
                window.request_redraw();
            }
            _ => {}
        }
    });

}