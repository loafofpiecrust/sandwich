use crate::sandwich::Ingredient;
use piston_window::*;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

pub fn setup_display<'a>() -> Sender<Vec<Ingredient>> {
    let (sender, receiver) = channel::<Vec<Ingredient>>();
    thread::spawn(move || {
        let mut window: PistonWindow = WindowSettings::new("SANDWICH", (1920, 1080))
            // .fullscreen(true)
            .automatic_close(true)
            .exit_on_esc(true)
            .build()
            .unwrap();
        let mut tc = TextureContext {
            factory: window.factory.clone(),
            encoder: window.factory.create_command_buffer().into(),
        };
        let offset = 20.0;
        let mut textures = Vec::new();
        while let Some(e) = window.next() {
            // Try to receive ingredient updates if there are any.
            if let Ok(ingredients) = receiver.try_recv() {
                textures = ingredients
                    .iter()
                    .map(|x| {
                        Texture::from_path(
                            &mut tc,
                            Path::new(&format!("images/{}.png", x.name())),
                            Flip::None,
                            &TextureSettings::new(),
                        )
                        .unwrap()
                    })
                    .collect();
            }
            window.draw_2d(&e, |c, g, d| {
                clear([0.5, 1.0, 0.5, 1.0], g);
                let mut curr = c.transform.trans(0.0, textures.len() as f64 * offset);
                for t in &textures {
                    image(t, curr, g);
                    curr = curr.trans(0.0, -offset);
                }
            });
        }
    });
    sender
}
