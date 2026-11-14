//! The dialog a scene is made in (OVRUN1, `mock_a.rs` page 7): a name, a background, a video and
//! music each chosen from a file dialog with a level slider, the logo, and what the scene takes
//! away when its video ends. Nothing in it is typed but the name.

use rune::{El, ScrollSpec, Tx, col, icon, row, spacer, text};
use rune_kit::{Checkbox, Dialog, Kit, ScrollBar, TextInput};

use super::calendar::field;
use super::plan::volume_slider;
use super::widgets::{btn, divider};
use crate::run::{Layer, Pick};
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, space, type_};

const WIDTH: f32 = 480.0;
/// The slider's length under a layer: the dialog less its padding and the layer's own indent.
const SLIDER: f32 = WIDTH - 2.0 * space::S4 - 28.0 - 44.0;

fn name_of(path: &str) -> String {
    std::path::Path::new(path).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

/// One layer that holds a file: its kind, the file or that none is chosen, a button to choose
/// one and one to take it away, and for a sound its level.
fn layer_row(k: &Kit, key: &str, glyph: &'static rune::Icon, words: &'static str, file: Option<&str>, layer: Layer, volume: Option<u8>) -> El {
    let choose = btn(t::BTN_CHOOSE()).small().focusable().build(k, move |s, _| s.run.pick = Some(Pick::Layer(layer))).key(format!("{key}-choose"));
    let remove = btn(t::BTN_REMOVE_PICTURE()).icon(&icons::CLOSE).icon_only().ghost().small().focusable().disabled(file.is_none()).build(k, move |s, _| s.scene_clear(layer)).key(format!("{key}-remove"));
    let value = match file {
        Some(path) => text(name_of(path)).font(type_::SMALL).color(k.c.text_base),
        None => text(t::LOOK_NO_FILE()).font(type_::SMALL).color(k.c.text_muted),
    };
    let head = row().w_full().gap(space::S2).center_y().child(icon(glyph, 14.0).color(k.c.text_muted).a11y_hidden()).child(text(words).font(type_::BODY_STRONG).color(k.c.text_base).w(92.0)).child(col().grow(1.0).min_w(0.0).child(value.truncate().min_w(0.0))).child(choose).child(remove);
    let mut block = col().key(key.to_string()).w_full().gap(space::S1).child(head);
    if let (Some(volume), Some(_)) = (volume, file) {
        let slider = volume_slider(k, format!("{key}-volume"), volume, SLIDER, move |s, to| s.scene_volume(layer, to));
        block = block.child(row().w_full().pl(22.0).gap(space::S2).center_y().child(slider).child(text(format!("{volume}%")).font(type_::TINY.features("'tnum' 1")).color(k.c.text_muted).w(34.0)));
    }
    block
}

pub fn dialog(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let Some(form) = s.run.form.as_ref() else { return };
    let name = TextInput::new(&form.name)
        .width(None)
        .label(t::RUN_SCENE_NAME())
        .on_edit(|s: &mut App| match s.run.form.as_mut() {
            Some(form) => &mut form.name,
            None => &mut s.spare_edit,
        })
        .on_submit(|s: &mut App, _: &mut Tx| s.save_scene())
        .build(k);
    let clip = |clip: &Option<artosia_core::plan::Clip>| clip.as_ref().map(|clip| clip.path.clone());
    let (video, music) = (clip(&form.video), clip(&form.music));
    let logo = Checkbox::new(form.logo).label(t::RUN_LAYER_LOGO()).on_toggle(|s: &mut App| {
        if let Some(form) = s.run.form.as_mut() {
            form.logo = !form.logo;
        }
    });
    let takes = |on: bool, layer: Layer, words: &'static str| Checkbox::new(on).label(words).on_toggle(move |s: &mut App| s.scene_clears(layer, !on)).build(k);
    let clears = col().w_full().gap(space::S1).child(takes(form.clears.video, Layer::Video, t::RUN_LAYER_VIDEO())).child(takes(form.clears.music, Layer::Music, t::RUN_LAYER_MUSIC())).child(takes(form.clears.logo, Layer::Logo, t::RUN_LAYER_LOGO()));
    let body = col()
        .key("scene-form")
        .w_full()
        .px(space::S4)
        .pb(space::S3)
        .gap(space::S3)
        .child(field(k, t::RUN_SCENE_NAME(), name))
        .child(layer_row(k, "scene-background", &icons::IMAGE, t::RUN_LAYER_BACKGROUND(), Some(form.background.as_str()).filter(|path| !path.is_empty()), Layer::Background, None))
        .child(layer_row(k, "scene-video", &icons::CAMERA, t::RUN_LAYER_VIDEO(), video.as_deref(), Layer::Video, form.video.as_ref().map(|clip| clip.volume)))
        .child(layer_row(k, "scene-music", &icons::SPEAKER, t::RUN_LAYER_MUSIC(), music.as_deref(), Layer::Music, form.music.as_ref().map(|clip| clip.volume)))
        .child(logo.build(k))
        .child(field(k, t::RUN_CLEARS(), clears));
    let scrolling = col().w_full().max_h((tall - 84.0 - 60.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    let foot = row()
        .w_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S2)
        .center_y()
        .child(spacer())
        .child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.close_scene_form()))
        .child(btn(t::BTN_ADD_TO_PLAN()).primary().disabled(!s.scene_ready()).focusable().build(k, |s, _| s.save_scene()).key("scene-save"));
    Dialog::new("scene", t::RUN_SCENE_NEW()).open(true).width(WIDTH).fit().on_close(|s: &mut App| s.close_scene_form()).child(col().w_full().child(scrolling).child(divider(k)).child(foot)).build(k, layer);
}
