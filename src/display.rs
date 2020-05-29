use crate::sandwich::Ingredient;
use piston_window::*;
use std::collections::HashMap;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;

#[derive(Debug, Default)]
pub struct Render {
    pub ingredients: Vec<Ingredient>,
    pub subtitles: String,
}

pub type RenderSender = SyncSender<Render>;

// TODO Render both ingredients and subtitles.
pub fn setup_display<'a>() -> RenderSender {
    let (sender, receiver) = sync_channel::<Render>(1);
    thread::spawn(move || {
        let mut window = WindowSettings::new("SANDWICH", (1920, 1080))
            .fullscreen(true)
            .automatic_close(true)
            .exit_on_esc(true)
            .vsync(true)
            .build::<PistonWindow>();
        if let Ok(mut window) = window {
            let mut tc = TextureContext {
                factory: window.factory.clone(),
                encoder: window.factory.create_command_buffer().into(),
            };
            let scale = 1.0;
            let offset = 10.0;
            let mut font = window.load_font("assets/OpenSans-Regular.ttf").unwrap();
            let mut texture_map = HashMap::new();
            let mut textures = Vec::new();
            let mut subtitles = String::new();
            while let Some(e) = window.next() {
                window.draw_2d(&e, |c, g, d| {
                    // Try to receive render updates if there are any.
                    if let Ok(render) = receiver.try_recv() {
                        // Convert ingredient name to texture of "images/ingredient-name.png"
                        textures = render
                            .ingredients
                            .into_iter()
                            .map(|x| {
                                texture_map
                                    .entry(x.name.clone())
                                    .or_insert_with(|| {
                                        Texture::from_path(
                                            &mut tc,
                                            &format!("images/{}.png", x.name),
                                            Flip::None,
                                            &TextureSettings::new()
                                                .compress(true)
                                                .filter(Filter::Nearest),
                                        )
                                        .unwrap()
                                    })
                                    .clone()
                            })
                            .collect();
                        subtitles = render.subtitles;
                    }

                    clear([0.0, 0.0, 0.0, 1.0], g);

                    // Render the subtitles.
                    let sub_t = c.transform.trans(200.0, 800.0);
                    text([1.0, 1.0, 1.0, 1.0], 30, &subtitles, &mut font, sub_t, g).unwrap();
                    // Push all text to the screen.
                    font.factory.encoder.flush(d);

                    // Render all the ingredients as stacked images.
                    let mut curr = c.transform.trans(400.0, 200.0).scale(scale, scale);
                    for t in &textures {
                        image(t, curr, g);
                        curr = curr.trans(0.0, -offset);
                    }
                });
            }
        } else {
            // Dummy receiver if we can't do visuals.
            loop {
                receiver.recv().unwrap();
            }
        }
    });
    sender
}
