use crate::sandwich::{Ingredient, Sandwich};
use piston_window::glyph_cache::rusttype::GlyphCache;
use piston_window::*;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[derive(Debug)]
pub struct Render {
    pub ingredients: Vec<Ingredient>,
    pub subtitles: String,
}

// TODO Render both ingredients and subtitles.
pub fn setup_display<'a>() -> Sender<Render> {
    let (sender, receiver) = channel::<Render>();
    thread::spawn(move || {
        let mut window: PistonWindow = WindowSettings::new("SANDWICH", (1920, 1080))
            .fullscreen(true)
            .automatic_close(true)
            .exit_on_esc(true)
            .build()
            .unwrap();
        let mut tc = TextureContext {
            factory: window.factory.clone(),
            encoder: window.factory.create_command_buffer().into(),
        };
        let scale = 1.0;
        let offset = 20.0 / scale;
        let mut font = window.load_font("assets/OpenSans-Regular.ttf").unwrap();
        let mut textures = Vec::new();
        let mut subtitles = String::new();
        while let Some(e) = window.next() {
            // Try to receive render updates if there are any.
            if let Ok(render) = receiver.try_recv() {
                println!("{:?}", render);
                // Convert ingredient name to texture of "images/ingredient-name.png"
                textures = render
                    .ingredients
                    .iter()
                    .map(|x| {
                        Texture::from_path(
                            &mut tc,
                            &format!("images/{}.png", x.name()),
                            Flip::None,
                            &TextureSettings::new().filter(Filter::Nearest),
                        )
                        .unwrap()
                    })
                    .collect();
                subtitles = render.subtitles;
            }
            window.draw_2d(&e, |c, g, d| {
                clear([0.0, 0.0, 0.0, 1.0], g);

                // Render the subtitles.
                let sub_t = c.transform.trans(50.0, 700.0);
                text([1.0, 1.0, 1.0, 1.0], 30, &subtitles, &mut font, sub_t, g).unwrap();
                // Push all text to the screen.
                font.factory.encoder.flush(d);

                // Render all the ingredients as stacked images.
                let mut curr = c
                    .transform
                    .trans(20.0, textures.len() as f64 * offset + 100.0)
                    .scale(scale, scale);
                for t in &textures {
                    image(t, curr, g);
                    curr = curr.trans(0.0, -offset);
                }
            });
        }
    });
    sender
}
