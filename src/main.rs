// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::{asset::AssetMetaCheck, input::mouse::MouseMotion, prelude::*, window::CursorGrabMode};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

#[derive(Component, Reflect)]
pub struct ActiveCamera {
    distance_to_focus: f32,
    rotation_y: f32,
    rotation_x: f32,
}

impl Default for ActiveCamera {
    fn default() -> Self {
        Self {
            distance_to_focus: 10.0,
            rotation_y: 0.0,
            rotation_x: 0.0,
        }
    }
}

#[derive(Resource)]
pub struct CameraControllerState {
    pub active: bool,
    pub mouse_speed: f32,
}
impl Default for CameraControllerState {
    fn default() -> Self {
        Self {
            active: false,
            mouse_speed: 0.1,
        }
    }
}

#[derive(Component)]
pub struct CameraFocus;

fn main() {
    let mut app = App::new();
    app.insert_resource(AssetMetaCheck::Never)
        .add_plugins(DefaultPlugins);

    if cfg!(debug_assertions) {
        app.add_plugins(WorldInspectorPlugin::new());
    }

    app.add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                camera_transform_update,
                camera_controller.pipe(error_handler),
            ),
        )
        .run();
}

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 3.0, 10.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..Default::default()
        },
        Name::new("Camera"),
        ActiveCamera::default(),
    ));

    commands.spawn((
        SceneBundle {
            scene: asset_server.load("ship.glb#Scene0"),
            ..Default::default()
        },
        Name::new("Ship"),
        CameraFocus,
    ));
    commands.insert_resource(AmbientLight {
        color: Color::ALICE_BLUE,
        brightness: 0.8,
    });
    commands.insert_resource(ClearColor(Color::BLACK));
    commands.insert_resource(CameraControllerState::default());
}

pub fn camera_transform_update(
    mut camera_query: Query<(&mut Transform, &ActiveCamera), Without<CameraFocus>>,
    camera_focus_query: Query<&Transform, With<CameraFocus>>,
) {
    if let Ok((mut camera_transform, camera)) = camera_query.get_single_mut() {
        if let Ok(camera_focus_transform) = camera_focus_query.get_single() {
            let camera_offset_x =
                camera.rotation_y.sin() * camera.distance_to_focus * camera.rotation_x.cos();
            let camera_offset_y = camera.rotation_x.sin() * camera.distance_to_focus;
            let camera_offset_z =
                camera.rotation_y.cos() * camera.distance_to_focus * camera.rotation_x.cos();

            camera_transform.translation = camera_focus_transform.translation
                + Vec3::new(camera_offset_x, camera_offset_y, camera_offset_z);
            camera_transform.look_at(camera_focus_transform.translation, Vec3::Y);
        }
    }
}

pub fn camera_controller(
    mut windows: Query<&mut Window>,
    mouse: Res<Input<MouseButton>>,
    mut camera_controller_state: ResMut<CameraControllerState>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    time: Res<Time>,
    mut camera_query: Query<&mut ActiveCamera>,
) -> anyhow::Result<()> {
    let mut window = windows.get_single_mut()?;
    if mouse.just_pressed(MouseButton::Left) {
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
        camera_controller_state.active = true;
    }
    if mouse.just_released(MouseButton::Left) {
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
        camera_controller_state.active = false;
    }
    for event in mouse_motion_events.read() {
        if camera_controller_state.active {
            bevy::log::info!("Mouse motion: {:?}", event);
            if let Ok(mut camera) = camera_query.get_single_mut() {
                camera.rotation_y -=
                    time.delta_seconds() * event.delta.x * camera_controller_state.mouse_speed;
                camera.rotation_x +=
                    time.delta_seconds() * event.delta.y * camera_controller_state.mouse_speed;
            }
        }
    }

    Ok(())
}

pub fn error_handler(In(result): In<anyhow::Result<()>>) {
    if let Err(e) = result {
        bevy::log::error!("Error: {}", e);
    }
}
