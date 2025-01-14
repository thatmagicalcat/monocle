use glfw::*;
use x11::xlib;

use stb_image::stb_image as image;

mod shader;
use shader::Shader;

fn main() {
    let ((screenshot_width, screenshot_height), screenshot_data) = screenshot();
    let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();

    glfw.window_hint(WindowHint::ContextVersionMinor(3));
    glfw.window_hint(WindowHint::ContextVersionMajor(3));
    glfw.window_hint(WindowHint::OpenGlProfile(OpenGlProfileHint::Core));

    let (mut window, events) = glfw.with_primary_monitor(|glfw, m| {
        let m = m.unwrap();
        glfw.create_window(1920, 1080, "Hello world", WindowMode::FullScreen(m))
            .expect("Failed to create GLFW window")
    });

    window.set_key_polling(true);
    window.set_mouse_button_polling(true);
    window.set_scroll_polling(true);

    gl::load_with(|s| window.get_proc_address(s));
    glfw.set_swap_interval(SwapInterval::Sync(1));

    // println!("OpenGL version: {}", unsafe {
    //     std::ffi::CStr::from_ptr(gl::GetString(gl::VERSION) as _).to_string_lossy()
    // });

    let size = window.get_size();
    let (sw, sh) = (size.0 as f32, size.1 as f32);

    unsafe { gl::Viewport(0, 0, sw as _, sh as _) };

    window.set_framebuffer_size_callback(|_, w, h| unsafe { gl::Viewport(0, 0, w, h) });

    let shader = Shader::from_str(include_str!("basic.glsl"), "vertex", "fragment").unwrap();

    let mut vao = 0;
    let mut vbo = 0;
    let mut ebo = 0;
    #[rustfmt::skip]
    let vertices: &[f32] = &[
        // position    // texture
         sw,  sh,      1.0, 1.0, // top right
         sw, 0.0,      1.0, 0.0, // bottom right
        0.0, 0.0,      0.0, 0.0, // bottom left
        0.0,  sh,      0.0, 1.0, // top left
    ];

    #[rustfmt::skip]
    let indices: &[u32] = &[
        0, 1, 3,
        1, 2, 3,
    ];

    unsafe {
        gl::GenVertexArrays(1, &raw mut vao);
        gl::GenBuffers(1, &raw mut vbo);
        gl::GenBuffers(1, &raw mut ebo);

        gl::BindVertexArray(vao);

        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            std::mem::size_of_val(indices) as _,
            indices.as_ptr() as _,
            gl::STATIC_DRAW,
        );

        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            std::mem::size_of_val(vertices) as _,
            vertices.as_ptr() as _,
            gl::STATIC_DRAW,
        );

        // position attribute
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0,
            2,
            gl::FLOAT,
            gl::FALSE,
            4 * std::mem::size_of::<f32>() as i32,
            std::ptr::null(),
        );

        // texture coordinate attribute
        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            4 * std::mem::size_of::<f32>() as i32,
            (2 * std::mem::size_of::<f32>()) as _,
        );

        // unbind
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);
    }

    // texture stuff
    unsafe { image::stbi_set_flip_vertically_on_load(true as _) };

    let mut texture1 = 0;
    unsafe {
        gl::GenTextures(1, &raw mut texture1);
        gl::BindTexture(gl::TEXTURE_2D, texture1);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB as _,
            screenshot_width,
            screenshot_height,
            0, // legacy stuff
            gl::RGB,
            gl::UNSIGNED_BYTE,
            screenshot_data.as_ptr() as _,
        );

        gl::GenerateMipmap(gl::TEXTURE_2D);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as _);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as _);

        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR_MIPMAP_LINEAR as _,
        );

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
    }

    let mut zoom: f32 = 1.0;
    let mut zoom_velocity = 0.0;

    // used for paning the screenshot
    let mut origin = (0.0, 0.0);

    // the point where the left mouse button was held
    let mut click_start_pos = None;

    while !window.should_close() {
        window.swap_buffers();

        glfw.poll_events();
        glfw::flush_messages(&events).for_each(|(_, event)| match event {
            WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),

            WindowEvent::Key(Key::R, _, Action::Press, _) => {
                origin = (0.0, 0.0);
                zoom = 1.0;
                zoom_velocity = 0.0;
            }

            WindowEvent::Scroll(_, delta) => zoom_velocity += 0.01 * delta as f32,
            WindowEvent::MouseButton(MouseButtonLeft, Action::Press, ..) => {
                click_start_pos = Some(window.get_cursor_pos());
            }

            WindowEvent::MouseButton(MouseButtonLeft, Action::Release, ..) => {
                click_start_pos = None;
            }

            _ => {}
        });

        let current_mouse_pos = window.get_cursor_pos();
        if let Some(start_pos) = click_start_pos {
            let displacement = (
                current_mouse_pos.0 - start_pos.0,
                current_mouse_pos.1 - start_pos.1,
            );

            origin.0 += displacement.0 as f32;
            origin.1 += displacement.1 as f32;

            match current_mouse_pos {
                (0.0, y) => window.set_cursor_pos(sw as _, y),
                (x, 0.0) => window.set_cursor_pos(x, sh as _),
                (x, y) if x + 1.0 == sw as f64 => window.set_cursor_pos(0.0, y),
                (x, y) if y + 1.0 == sh as f64 => window.set_cursor_pos(x, 0.0),

                _ => {}
            }

            if current_mouse_pos.0 == 0.0 {
                window.set_cursor_pos(sw as _, current_mouse_pos.1);
            }

            click_start_pos = Some(window.get_cursor_pos());
        }

        zoom += zoom_velocity;
        zoom_velocity *= 0.95; // damping

        zoom = zoom.clamp(0.01, 100.0);

        let center_x = sw / 2.0;
        let center_y = sh / 2.0;
        let left = center_x - (center_x / zoom) - origin.0 / zoom;
        let right = center_x + (center_x / zoom) - origin.0 / zoom;
        let bottom = center_y - (center_y / zoom) + origin.1 / zoom;
        let top = center_y + (center_y / zoom) + origin.1 / zoom;

        let orthographic_projection =
            glam::Mat4::orthographic_rh_gl(left, right, bottom, top, -1.0, 1.0);

        unsafe {
            gl::ClearColor(0.2, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            shader.use_shader();
            let uniform_loc = shader.get_uniform_location("projection").unwrap();

            gl::UniformMatrix4fv(
                uniform_loc,
                1,
                gl::FALSE,
                orthographic_projection.as_ref().as_ptr(),
            );

            gl::BindVertexArray(vao);
            gl::BindTexture(gl::TEXTURE_2D, texture1);
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
        }
    }

    // cleanup
    unsafe {
        gl::DeleteVertexArrays(1, &raw mut vao);
        gl::DeleteBuffers(1, &raw mut vbo);
        gl::DeleteBuffers(1, &raw mut ebo);
        gl::DeleteTextures(1, &raw mut texture1);
    }
}

/// returns: ((width, height), data)
fn screenshot() -> ((i32, i32), Vec<u8>) {
    let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };
    let screen = unsafe { xlib::XDefaultScreen(display) };
    let root = unsafe { xlib::XRootWindow(display, screen) };

    let height = unsafe { xlib::XDisplayHeight(display, screen) };
    let width = unsafe { xlib::XDisplayWidth(display, screen) };

    let image = unsafe {
        xlib::XGetImage(
            display,
            root,
            0,
            0,
            width as _,
            height as _,
            xlib::XAllPlanes(),
            xlib::ZPixmap,
        )
    };

    assert!(!image.is_null());

    let visual = unsafe { xlib::XDefaultVisual(display, screen) };
    let (red_mask, green_mask, blue_mask) = unsafe {
        let v = *visual;
        (v.red_mask, v.green_mask, v.blue_mask)
    };

    let mut buf: Vec<u8> = vec![0; (width * height * 3) as usize];
    for y in 0..height {
        for x in 0..width {
            let pixel = unsafe { xlib::XGetPixel(image, x, y) };

            let rgb: [u8; 3] = [
                ((pixel & red_mask) >> red_mask.trailing_zeros()) as _,
                ((pixel & green_mask) >> green_mask.trailing_zeros()) as _,
                ((pixel & blue_mask) >> blue_mask.trailing_zeros()) as _,
            ];

            // Calculate the index for the flipped image
            let index = (((height - 1 - y) * width + x) * 3) as usize;
            buf[index..index + 3].copy_from_slice(&rgb);
        }
    }

    unsafe {
        xlib::XDestroyImage(image);
        xlib::XCloseDisplay(display);
    }

    ((width, height), buf)
}
