// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::{asset::AssetMetaCheck, input::mouse::MouseMotion, prelude::*, window::CursorGrabMode};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::prelude::*;
use rand::prelude::*;

#[derive(Resource)]
pub struct BulletAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

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
    pub bullet_spawn_distance: f32,
}

#[derive(Component)]
pub struct Bullet;

#[derive(Bundle)]
pub struct StarBundle {
    pub pbr_bundle: PbrBundle,
    pub star: Star,
}

#[derive(Event)]
pub struct ShootBulletEvent {
    shooter: Entity,
}

#[derive(Bundle)]
pub struct BulletBundle {
    pub pbr_bundle: PbrBundle,
    pub bullet: Bullet,
    pub collider: Collider,
    pub rigid_body: RigidBody,
    pub velocity: Velocity,
}
impl BulletBundle {
    pub fn new(bullet_assets: &BulletAssets, direction: Vec3, transform: Transform) -> Self {
        Self {
            pbr_bundle: PbrBundle {
                mesh: bullet_assets.mesh.clone(),
                material: bullet_assets.material.clone(),
                transform,
                ..Default::default()
            },
            bullet: Bullet,
            collider: Collider::ball(0.1),
            rigid_body: RigidBody::Dynamic,
            velocity: Velocity::linear(direction * 100.0),
        }
    }
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
        .add_event::<ShootBulletEvent>()
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
                ship_velocity_controller,
                respawn_stars.pipe(error_handler),
                player_rotation_controller.pipe(error_handler),
                spawn_bullet,
                autoshoot.pipe(error_handler),
            ),
        )
        .run();
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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
        Ship {
            speed: 10.0,
            bullet_spawn_distance: 2.0,
        },
        Player,
        Collider::cuboid(1.0, 0.4, 1.0),
        RigidBody::Dynamic,
        Velocity::default(),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::ALICE_BLUE,
        brightness: 0.8,
    });

    commands.spawn((
        SceneBundle {
            scene: asset_server.load("enemy-ship.glb#Scene0"),
            transform: Transform::from_xyz(0.0, 0.0, -50.0),
            ..Default::default()
        },
        Name::new("Enemy"),
        Ship {
            speed: 5.0,
            bullet_spawn_distance: 2.0,
        },
        Velocity::default(),
        Collider::cuboid(2.5, 0.6, 1.5),
        RigidBody::Dynamic,
    ));
    let bullet_shape = meshes.add(
        shape::UVSphere {
            radius: 0.3,
            ..Default::default()
        }
        .into(),
    );
    let bullet_material = materials.add(StandardMaterial {
        base_color: Color::RED,
        unlit: true,
        ..Default::default()
    });
    let bullet_assets = BulletAssets {
        mesh: bullet_shape,
        material: bullet_material,
    };

    commands.insert_resource(ClearColor(Color::BLACK));
    commands.insert_resource(CameraControllerState::default());
    commands.insert_resource(bullet_assets);
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

pub fn ship_velocity_controller(mut player_query: Query<(&mut Velocity, &Transform, &Ship)>) {
    for (mut velocity, transform, ship) in player_query.iter_mut() {
        velocity.linvel = transform.rotation * Vec3::new(0.0, 0.0, -ship.speed);
    }
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

pub fn spawn_bullet(
    mut commands: Commands,
    mut bullet_event_reader: EventReader<ShootBulletEvent>,
    ship_query: Query<(&Transform, &Ship)>,
    bullet_assets: Res<BulletAssets>,
) {
    for bullet_event in bullet_event_reader.read() {
        if let Ok((transform, ship)) = ship_query.get(bullet_event.shooter) {
            let forward_vector = transform.forward();
            let bullet_spawn_offset = forward_vector * ship.bullet_spawn_distance;
            let bullet = BulletBundle::new(
                &bullet_assets.as_ref(),
                forward_vector,
                Transform::from_translation(transform.translation + bullet_spawn_offset),
            );
            commands.spawn(bullet);
        }
    }
}

pub fn autoshoot(
    mut bullet_event_writer: EventWriter<ShootBulletEvent>,
    player_query: Query<Entity, With<Player>>,
    mut timer: Local<Option<Timer>>,
    time: Res<Time>,
) -> anyhow::Result<()> {
    if timer.is_none() {
        *timer = Some(Timer::from_seconds(3.0, TimerMode::Repeating));
    }
    let timer = timer.as_mut().unwrap();
    if timer.tick(time.delta()).just_finished() {
        let player_entity = player_query.get_single()?;
        bullet_event_writer.send(ShootBulletEvent {
            shooter: player_entity,
        });
    }

    Ok(())
}

pub fn error_handler(In(result): In<anyhow::Result<()>>) {
    if let Err(e) = result {
        bevy::log::error!("Error: {}", e);
    }
}
