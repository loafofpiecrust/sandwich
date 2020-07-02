use crate::behavior::{personality, Personality};
use crate::sandwich::Ingredient;
use async_std::task;
use piston_window::*;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::{thread, time::Instant};

#[derive(Debug, Default)]
pub struct Render {
    pub ingredients: Option<Vec<Ingredient>>,
    pub subtitles: Option<String>,
    pub background: Option<&'static str>,
}
impl Render {
    pub fn clear() -> Self {
        Self {
            ingredients: Some(Default::default()),
            subtitles: Some(Default::default()),
            background: Some("000000ff"),
        }
    }
}

pub struct Display {
    pub render: RenderSender,
    pub actions: Receiver<PersonalityAction>,
    pub keys: Receiver<Button>,
}

pub type PersonalityAction = fn(&mut Personality) -> ();
pub type RenderSender = SyncSender<Render>;

// TODO Render both ingredients and subtitles.
pub fn setup_display<'a>() -> Display {
    let (sender, receiver) = sync_channel::<Render>(1);
    let (action_sx, action_rx) = sync_channel::<PersonalityAction>(1);
    let (key_sx, key_rx) = sync_channel(1);
    task::spawn(async move {
        let window = std::panic::catch_unwind(|| {
            WindowSettings::new("SANDWICH", (1920, 1080))
                .fullscreen(true)
                .automatic_close(true)
                .exit_on_esc(true)
                .vsync(true)
                .build::<PistonWindow>()
                .expect("Failed to build window")
        });
        if let Ok(mut window) = window {
            let mut events = Events::new(EventSettings::new());
            let mut tc = TextureContext {
                factory: window.factory.clone(),
                encoder: window.factory.create_command_buffer().into(),
            };
            let scale = 12.0;
            let offset = 15.0;
            let mut rng = thread_rng();
            let mut font = window
                .load_font("assets/OpenSans-Regular.ttf")
                .expect("Failed to load font");
            let mut texture_map = HashMap::new();
            let mut textures = Vec::new();
            let mut rotations = Vec::<f64>::new();
            let mut subtitles = String::new();
            let mut background = [0.0, 0.0, 0.0, 1.0];
            while let Some(e) = events.next(&mut window) {
                // Try to receive render updates if there are any.
                if let Ok(render) = receiver.try_recv() {
                    // Convert ingredient name to texture of "images/ingredient-name.png"
                    if let Some(ingr) = render.ingredients {
                        if rotations.len() > ingr.len() {
                            rotations.truncate(ingr.len());
                        } else {
                            while rotations.len() < ingr.len() {
                                rotations.push(rng.gen_range(-15.0, 15.0));
                            }
                        }

                        textures = ingr
                            .into_iter()
                            .map(|x| {
                                texture_map
                                    .entry(x.name.clone())
                                    .or_insert_with(|| {
                                        println!("{}", x.name);
                                        Texture::from_path(
                                            &mut tc,
                                            &format!("images/{}.png", x.name),
                                            Flip::None,
                                            &TextureSettings::new()
                                                .compress(true)
                                                .filter(Filter::Nearest),
                                        )
                                        .expect("Failed to open ingredient image")
                                    })
                                    .clone()
                            })
                            .collect();
                    }
                    if let Some(subs) = render.subtitles {
                        subtitles = subs;
                    }
                    if let Some(bg) = render.background {
                        background = piston_window::color::hex(bg);
                    }
                }
                window.draw_2d(&e, |c, g, d| {
                    clear(background, g);

                    // Render the subtitles.
                    let sub_t = c.transform.trans(200.0, 900.0);
                    text([1.0, 1.0, 1.0, 1.0], 40, &subtitles, &mut font, sub_t, g).unwrap();
                    // Push all text to the screen.
                    font.factory.encoder.flush(d);

                    // Render all the ingredients as stacked images.
                    for (idx, t) in textures.iter().enumerate() {
                        let rot = if idx == 0 || idx == textures.len() - 1 {
                            0.0
                        } else {
                            rotations[idx]
                        };
                        let transform = c
                            .transform
                            .trans(960.0, 600.0 - offset * idx as f64)
                            .rot_deg(rot)
                            .scale(scale, scale)
                            // Anchor elements at theit center point.
                            .trans(-16.0, -16.0);
                        image(t, transform, g);
                    }
                });

                // Add some keybindings for testing out real-time interaction.
                if let Some(k) = e.button_args() {
                    if k.state == ButtonState::Press {
                        println!("key press: {:?}", k);
                        key_sx
                            .send(k.button)
                            .expect("Failed to send key press from window");
                    }
                };
            }
        } else {
            // Dummy receiver if we can't do visuals.
            loop {
                receiver.recv().unwrap();
            }
        }
    });

    Display {
        render: sender,
        actions: action_rx,
        keys: key_rx,
    }
}
