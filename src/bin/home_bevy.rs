use bevy::prelude::*;
use bevy::window::{Window, WindowPlugin};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::WHITE))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Jinof's Blog".to_string(),
                canvas: Some("#bevy-home-canvas".to_string()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let ink = Color::srgb(0.08, 0.09, 0.10);
    let ground = Color::srgba(0.08, 0.09, 0.10, 0.30);
    let shadow = Color::srgba(0.0, 0.0, 0.0, 0.12);

    let pelvis = Vec2::new(0.0, -66.0);
    let shoulder = pelvis + Vec2::new(0.0, 98.0);
    let head_center = shoulder + Vec2::new(0.0, 49.0);

    let left_elbow = shoulder + Vec2::new(-38.0, -34.0);
    let left_hand = left_elbow + Vec2::new(-34.0, -33.0);
    let right_elbow = shoulder + Vec2::new(34.0, 28.0);
    let right_hand = right_elbow + Vec2::new(31.0, 39.0);

    let left_knee = pelvis + Vec2::new(-24.0, -57.0);
    let left_foot = pelvis + Vec2::new(-42.0, -118.0);
    let right_knee = pelvis + Vec2::new(23.0, -55.0);
    let right_foot = pelvis + Vec2::new(45.0, -118.0);

    commands.spawn((
        Sprite::from_color(shadow, Vec2::new(1.0, 1.0)),
        Transform {
            translation: Vec3::new(0.0, -193.0, 0.0),
            scale: Vec3::new(150.0, 7.0, 1.0),
            ..default()
        },
    ));

    spawn_line(
        &mut commands,
        Vec2::new(-310.0, -205.0),
        Vec2::new(310.0, -205.0),
        3.0,
        1.0,
        ground,
    );

    spawn_line(&mut commands, pelvis, shoulder, 7.0, 4.0, ink);
    spawn_head(&mut commands, head_center, 26.0, ink);
    spawn_line(&mut commands, shoulder, left_elbow, 6.0, 4.0, ink);
    spawn_line(&mut commands, left_elbow, left_hand, 6.0, 4.0, ink);
    spawn_line(&mut commands, shoulder, right_elbow, 6.0, 4.0, ink);
    spawn_line(&mut commands, right_elbow, right_hand, 6.0, 4.0, ink);
    spawn_line(&mut commands, pelvis, left_knee, 6.5, 4.0, ink);
    spawn_line(&mut commands, left_knee, left_foot, 6.5, 4.0, ink);
    spawn_line(&mut commands, pelvis, right_knee, 6.5, 4.0, ink);
    spawn_line(&mut commands, right_knee, right_foot, 6.5, 4.0, ink);
}

fn spawn_head(commands: &mut Commands, center: Vec2, radius: f32, color: Color) {
    let points = [
        center + Vec2::new(-radius * 0.5, radius),
        center + Vec2::new(radius * 0.5, radius),
        center + Vec2::new(radius, 0.0),
        center + Vec2::new(radius * 0.5, -radius),
        center + Vec2::new(-radius * 0.5, -radius),
        center + Vec2::new(-radius, 0.0),
    ];

    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        spawn_line(commands, points[index], points[next], 5.5, 5.0, color);
    }
}

fn spawn_line(
    commands: &mut Commands,
    start: Vec2,
    end: Vec2,
    thickness: f32,
    z: f32,
    color: Color,
) {
    commands.spawn((
        Sprite::from_color(color, Vec2::new(1.0, 1.0)),
        line_transform(start, end, thickness, z),
    ));
}

fn line_transform(start: Vec2, end: Vec2, thickness: f32, z: f32) -> Transform {
    let delta = end - start;
    let length = delta.length().max(1.0);
    let midpoint = (start + end) * 0.5;

    Transform {
        translation: Vec3::new(midpoint.x, midpoint.y, z),
        rotation: Quat::from_rotation_z(delta.y.atan2(delta.x)),
        scale: Vec3::new(length, thickness, 1.0),
    }
}
