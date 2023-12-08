// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::{asset::AssetMetaCheck, input::mouse::MouseMotion, prelude::*, window::CursorGrabMode};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::prelude::*;
use rand::prelude::*;

#[derive(Component, Reflect)]
pub struct ActiveCamera {
    pub distance_to_focus: f32,
    pub rotation_y: f32,
    pub rotation_x: f32,
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

#[derive(Resource, Reflect)]
#[reflect(Resource)]
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

#[derive(Component)]
pub struct Star;

#[derive(Component)]
pub struct Player;
#[derive(Component)]
pub struct Ship {
    pub speed: f32,
}

#[derive(Bundle)]
pub struct StarBundle {
    pub pbr_bundle: PbrBundle,
    pub star: Star,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(AssetMetaCheck::Never)
        .add_plugins(DefaultPlugins)
        .register_type::<ActiveCamera>()
        .register_type::<CameraControllerState>()
        .insert_resource(RapierConfiguration {
            gravity: Vec3::new(0.0, 0.0, 0.0),
            ..Default::default()
        })
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default());

    if cfg!(debug_assertions) {
        app.add_plugins(WorldInspectorPlugin::new());
    }

    app.add_systems(Startup, (setup, setup_stars))
        .add_systems(
            Update,
            (
                camera_transform_update,
                camera_controller.pipe(error_handler),
                player_velocity_controller.pipe(error_handler),
                respawn_stars.pipe(error_handler),
                player_rotation_controller.pipe(error_handler),
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
        Ship { speed: 10.0 },
        Player,
        Collider::cuboid(1.0, 0.4, 1.0),
        RigidBody::Dynamic,
        Velocity::default(),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::ALICE_BLUE,
        brightness: 0.8,
    });

    commands.insert_resource(ClearColor(Color::BLACK));
    commands.insert_resource(CameraControllerState::default());
}

pub fn setup_stars(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let star_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..Default::default()
    });
    let star_shape = meshes.add(
        shape::UVSphere {
            radius: 0.3,
            ..Default::default()
        }
        .into(),
    );
    commands.spawn_batch((0..1000).map(move |_| {
        let mut rng = rand::thread_rng();
        let phi = rng.gen_range(0.0..std::f32::consts::PI * 2.0);
        let theta = rng.gen_range(0.0..std::f32::consts::PI * 2.0);
        let distance = rng.gen_range(20.0..100.0);
        let x = distance * phi.sin() * theta.cos();
        let y = distance * phi.sin() * theta.sin();
        let z = distance * phi.cos();
        let mut transform = Transform::from_translation(Vec3::new(x, y, z));
        transform.scale = Vec3::splat(0.1);
        StarBundle {
            pbr_bundle: PbrBundle {
                mesh: star_shape.clone(),
                material: star_material.clone(),
                transform,
                ..Default::default()
            },
            star: Star,
        }
    }));
}

pub fn respawn_stars(
    mut star_query: Query<&mut Transform, (With<Star>, Without<ActiveCamera>)>,
    camera_query: Query<&Transform, With<ActiveCamera>>,
) -> anyhow::Result<()> {
    let camera_transform = camera_query.get_single()?;
    for mut star_transform in star_query.iter_mut() {
        if star_transform
            .translation
            .distance(camera_transform.translation)
            > 100.0
        {
            //let mut rng = rand::thread_rng();
            //let x = rng.gen_range(-100.0..100.0);
            //let y = rng.gen_range(-100.0..100.0);
            //let z = rng.gen_range(-100.0..100.0);
            let diff = star_transform.translation - camera_transform.translation;
            star_transform.translation = camera_transform.translation - diff;
        }
    }

    Ok(())
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

pub fn player_velocity_controller(
    mut player_query: Query<(&mut Velocity, &Transform, &Ship), With<Player>>,
) -> anyhow::Result<()> {
    let (mut velocity, transform, ship) = player_query.get_single_mut()?;
    velocity.linvel = transform.rotation * Vec3::new(0.0, 0.0, -ship.speed);

    Ok(())
}

pub fn player_rotation_controller(
    mut player_query: Query<(&mut Transform, &mut Velocity, &Ship), With<Player>>,
    camera_query: Query<&Transform, (With<ActiveCamera>, Without<Player>)>,
) -> anyhow::Result<()> {
    let (mut player_transform, mut velocity, ship) = player_query.get_single_mut()?;
    let camera_transform = camera_query.get_single()?;
    player_transform.rotation =
        player_transform.rotation + (camera_transform.rotation - player_transform.rotation) * 0.2;

    Ok(())
}

pub fn error_handler(In(result): In<anyhow::Result<()>>) {
    if let Err(e) = result {
        bevy::log::error!("Error: {}", e);
    }
}
