//! Nightfall Studio — GPU-accelerated desktop editor built with gpui-kit.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use gpui_kit::component::button::*;
use gpui_kit::component::label::Label;
use gpui_kit::component::*;
use gpui_kit::*;
use nightfall_data::ProjectData;

const DEFAULT_ASSET_GEN_URL: &str =
    "https://semnyk-adamec--qwen38-sglang-runner-assetgenerator-generate.modal.run";

struct NightfallEditor {
    assets_dir: PathBuf,
    data: ProjectData,
    selected_enemy: usize,
    status: String,
}

impl NightfallEditor {
    fn new() -> Self {
        let assets_dir = ProjectData::default_assets_dir();
        let (data, status) = match ProjectData::load_from_dir(&assets_dir) {
            Ok(data) => (
                data,
                format!("Loaded project data from {}", assets_dir.display()),
            ),
            Err(err) => (
                ProjectData::default(),
                format!("Using empty project ({err})"),
            ),
        };
        Self {
            assets_dir,
            data,
            selected_enemy: 0,
            status,
        }
    }

    fn save_data(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        match self.data.save_to_dir(&self.assets_dir) {
            Ok(()) => self.status = format!("Saved JSON data to {}/data/", self.assets_dir.display()),
            Err(err) => self.status = format!("Save failed: {err}"),
        }
        cx.notify();
    }

    fn bump_selected_enemy_hp(&mut self, delta: f32, cx: &mut Context<Self>) {
        if let Some(enemy) = self.data.enemies.get_mut(self.selected_enemy) {
            enemy.hp = (enemy.hp + delta).max(5.0);
            self.status = format!("{} HP -> {:.0}", enemy.name, enemy.hp);
            cx.notify();
        }
    }

    fn generate_selected_sprite(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(enemy) = self.data.enemies.get(self.selected_enemy).cloned() else {
            self.status = "No enemy selected.".into();
            cx.notify();
            return;
        };

        let url = std::env::var("ASSET_GEN_URL").unwrap_or_else(|_| DEFAULT_ASSET_GEN_URL.into());
        let prompt = format!(
            "2d isometric dark fantasy game sprite of {}, hand-painted style, isolated on plain white background",
            enemy.name
        );

        let result = ureq::post(&url).send_json(serde_json::json!({
            "prompt": prompt,
            "width": 1024,
            "height": 1024,
            "remove_bg": true
        }));

        match result {
            Ok(resp) => {
                let mut bytes = Vec::new();
                if let Err(err) = resp.into_reader().read_to_end(&mut bytes) {
                    self.status = format!("Failed to read PNG stream: {err}");
                } else {
                    let dest = self.assets_dir.join(&enemy.sprite_path);
                    if let Some(parent) = dest.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    match fs::write(&dest, &bytes) {
                        Ok(()) => {
                            self.status = format!(
                                "Generated {} ({} bytes)",
                                dest.display(),
                                bytes.len()
                            );
                        }
                        Err(err) => self.status = format!("Write error: {err}"),
                    }
                }
            }
            Err(err) => self.status = format!("Modal AssetGenerator error: {err}"),
        }
        cx.notify();
    }

    fn launch_game(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let candidate = PathBuf::from("dist/nightfall-game");
        let bin = if candidate.exists() {
            candidate
        } else {
            PathBuf::from("nightfall-game")
        };

        match Command::new(&bin).spawn() {
            Ok(_) => self.status = format!("Launched {}", bin.display()),
            Err(err) => self.status = format!("Failed to launch {}: {err}", bin.display()),
        }
        cx.notify();
    }
}

impl Render for NightfallEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_summary = self
            .data
            .enemies
            .get(self.selected_enemy)
            .map(|e| {
                format!(
                    "{} (id: {}) — HP: {:.0}, Speed: {:.0}, Damage: {:.0}, Sprite: {}",
                    e.name, e.id, e.hp, e.speed, e.damage, e.sprite_path
                )
            })
            .unwrap_or_else(|| "No enemies defined in assets/data/enemies.json".into());

        let enemy_buttons = self.data.enemies.iter().enumerate().map(|(idx, enemy)| {
            let label = format!("{} (HP {:.0})", enemy.name, enemy.hp);
            Button::new(("enemy", idx))
                .label(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selected_enemy = idx;
                    this.status = format!("Selected {}", this.data.enemies[idx].name);
                    cx.notify();
                }))
        });

        div()
            .v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .child(
                div()
                    .h_flex()
                    .justify_between()
                    .items_center()
                    .child(Label::new("Nightfall Studio (gpui-kit)"))
                    .child(
                        div()
                            .h_flex()
                            .gap_2()
                            .child(
                                Button::new("save")
                                    .label("Save JSON")
                                    .on_click(cx.listener(|this, _, w, cx| this.save_data(w, cx))),
                            )
                            .child(
                                Button::new("play")
                                    .primary()
                                    .label("Play Game")
                                    .on_click(cx.listener(|this, _, w, cx| this.launch_game(w, cx))),
                            ),
                    ),
            )
            .child(
                div()
                    .h_flex()
                    .gap_4()
                    .flex_1()
                    .child(
                        div()
                            .v_flex()
                            .gap_2()
                            .w(px(260.0))
                            .child(Label::new("Enemies"))
                            .children(enemy_buttons),
                    )
                    .child(
                        div()
                            .v_flex()
                            .gap_3()
                            .flex_1()
                            .child(Label::new("Inspector"))
                            .child(Label::new(selected_summary))
                            .child(
                                div()
                                    .h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("hp-minus")
                                            .label("-10 HP")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.bump_selected_enemy_hp(-10.0, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("hp-plus")
                                            .label("+10 HP")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.bump_selected_enemy_hp(10.0, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("gen-sprite")
                                            .primary()
                                            .label("Generate 2D Sprite (Modal FLUX)")
                                            .on_click(cx.listener(|this, _, w, cx| {
                                                this.generate_selected_sprite(w, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(Label::new(format!("Status: {}", self.status)))
    }
}

fn main() {
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| NightfallEditor::new())
        })
        .expect("Failed to open Nightfall Studio window");
    });
}
