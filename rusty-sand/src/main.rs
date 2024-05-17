use nannou::prelude::*;

fn main() {
    nannou::app(model).update(update).simple_window(view).run();
}

struct Model {
}

fn model(_app: &App) -> Model {
    Model {
    }
}

fn update(_app: &App, _model: &mut Model, _update: Update) {

}

fn view(app: &App, _model: &Model, frame: Frame) {
    let win = app.window_rect();

    let draw = app.draw();
    draw.background().color(BLACK);
    draw.text((app.fps() as u32).to_string().as_str())
        .xy(win.top_left() + pt2(20.0, -20.0))
        .color(RED);
    draw.to_frame(app, &frame).unwrap();
}