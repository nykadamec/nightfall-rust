//! Nightfall 2D isometric survivor-like runtime built with Bevy.

use std::f32::consts::TAU;

use bevy::prelude::*;
use nightfall_data::{ProjectData, world_to_iso};

#[derive(Resource)]
struct GameData(ProjectData);

#[derive(Resource)]
struct SpawnTimer {
    timer: Timer,
    count: u32,
}

#[derive(Resource)]
struct WeaponTimer(Timer);

#[derive(Component)]
struct WorldPos(Vec2);

#[derive(Component)]
struct Player {
    speed: f32,
}

#[derive(Component)]
struct Enemy {
    speed: f32,
    hp: f32,
}

#[derive(Component)]
struct Projectile {
    velocity: Vec2,
    damage: f32,
    lifetime: f32,
}

fn main() {
    let assets_dir = ProjectData::default_assets_dir();
    let project_data = ProjectData::load_from_dir(&assets_dir).unwrap_or_else(|err| {
        eprintln!("Warning: could not load project data ({err}), using defaults.");
        ProjectData::default()
    });

    let spawn_interval = project_data
        .waves
        .first()
        .map(|w| w.spawn_interval_s.max(0.1))
        .unwrap_or(1.2);
    let weapon_cd = project_data
        .weapons
        .first()
        .map(|w| w.cooldown_s.max(0.1))
        .unwrap_or(0.65);

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.06, 0.06, 0.09)))
        .insert_resource(GameData(project_data))
        .insert_resource(SpawnTimer {
            timer: Timer::from_seconds(spawn_interval, TimerMode::Repeating),
            count: 0,
        })
        .insert_resource(WeaponTimer(Timer::from_seconds(
            weapon_cd,
            TimerMode::Repeating,
        )))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Nightfall (Bevy 2D Isometric)".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                player_movement,
                spawn_enemies,
                enemy_chase,
                auto_fire_weapon,
                move_projectiles,
                resolve_projectile_hits,
                sync_iso_transforms,
                camera_follow,
            )
                .chain(),
        )
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Subtle isometric ground grid markers.
    for gx in -8..=8 {
        for gy in -8..=8 {
            let (sx, sy) = world_to_iso(gx as f32 * 64.0, gy as f32 * 64.0);
            let shade = if (gx + gy) % 2 == 0 { 0.11 } else { 0.14 };
            commands.spawn((
                Sprite {
                    color: Color::srgb(shade, shade, shade + 0.03),
                    custom_size: Some(Vec2::new(60.0, 30.0)),
                    ..default()
                },
                Transform::from_xyz(sx, sy, -100.0),
            ));
        }
    }

    // Player entity.
    commands.spawn((
        Player { speed: 190.0 },
        WorldPos(Vec2::ZERO),
        Sprite {
            color: Color::srgb(0.95, 0.78, 0.28),
            custom_size: Some(Vec2::new(28.0, 40.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 10.0),
    ));
}

fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Player, &mut WorldPos)>,
) {
    let Ok((player, mut pos)) = query.get_single_mut() else {
        return;
    };

    let mut iso_dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        iso_dir.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        iso_dir.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        iso_dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        iso_dir.x += 1.0;
    }

    if iso_dir.length_squared() > 0.0 {
        let dir = iso_dir.normalize();
        // Map screen-space WASD cleanly into logical world coordinates.
        let world_delta = Vec2::new(dir.y + dir.x, dir.y - dir.x).normalize_or_zero();
        pos.0 += world_delta * player.speed * time.delta_secs();
    }
}

fn spawn_enemies(
    time: Res<Time>,
    game_data: Res<GameData>,
    mut spawner: ResMut<SpawnTimer>,
    player_q: Query<&WorldPos, With<Player>>,
    mut commands: Commands,
) {
    if !spawner.timer.tick(time.delta()).just_finished() {
        return;
    }
    let Ok(player_pos) = player_q.get_single() else {
        return;
    };

    let angle = (spawner.count as f32 * 2.39996) % TAU;
    spawner.count = spawner.count.wrapping_add(1);
    let offset = Vec2::new(angle.cos(), angle.sin()) * 360.0;
    let spawn_pos = player_pos.0 + offset;

    let def = game_data.0.enemies.first();
    let (hp, speed) = def.map(|e| (e.hp, e.speed)).unwrap_or((30.0, 95.0));

    commands.spawn((
        Enemy { speed, hp },
        WorldPos(spawn_pos),
        Sprite {
            color: Color::srgb(0.82, 0.24, 0.32),
            custom_size: Some(Vec2::new(24.0, 32.0)),
            ..default()
        },
        Transform::default(),
    ));
}

fn enemy_chase(
    time: Res<Time>,
    player_q: Query<&WorldPos, (With<Player>, Without<Enemy>)>,
    mut enemies_q: Query<(&Enemy, &mut WorldPos), Without<Player>>,
) {
    let Ok(player_pos) = player_q.get_single() else {
        return;
    };

    for (enemy, mut pos) in &mut enemies_q {
        let dir = (player_pos.0 - pos.0).normalize_or_zero();
        pos.0 += dir * enemy.speed * time.delta_secs();
    }
}

fn auto_fire_weapon(
    time: Res<Time>,
    game_data: Res<GameData>,
    mut weapon_timer: ResMut<WeaponTimer>,
    player_q: Query<&WorldPos, With<Player>>,
    enemies_q: Query<&WorldPos, With<Enemy>>,
    mut commands: Commands,
) {
    if !weapon_timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let Ok(player_pos) = player_q.get_single() else {
        return;
    };

    let nearest = enemies_q
        .iter()
        .min_by(|a, b| {
            let da = a.0.distance_squared(player_pos.0);
            let db = b.0.distance_squared(player_pos.0);
            da.total_cmp(&db)
        });

    let Some(target) = nearest else {
        return;
    };

    let weapon = game_data.0.weapons.first();
    let damage = weapon.map(|w| w.damage).unwrap_or(18.0);
    let proj_speed = weapon.map(|w| w.projectile_speed).unwrap_or(420.0);
    let dir = (target.0 - player_pos.0).normalize_or_zero();

    commands.spawn((
        Projectile {
            velocity: dir * proj_speed,
            damage,
            lifetime: 2.0,
        },
        WorldPos(player_pos.0),
        Sprite {
            color: Color::srgb(0.45, 0.85, 1.0),
            custom_size: Some(Vec2::new(10.0, 10.0)),
            ..default()
        },
        Transform::default(),
    ));
}

fn move_projectiles(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Projectile, &mut WorldPos)>,
) {
    let dt = time.delta_secs();
    for (entity, mut proj, mut pos) in &mut query {
        pos.0 += proj.velocity * dt;
        proj.lifetime -= dt;
        if proj.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn resolve_projectile_hits(
    mut commands: Commands,
    projectiles_q: Query<(Entity, &Projectile, &WorldPos)>,
    mut enemies_q: Query<(Entity, &mut Enemy, &WorldPos)>,
) {
    let hit_radius_sq = 26.0 * 26.0;
    for (proj_entity, proj, proj_pos) in &projectiles_q {
        for (enemy_entity, mut enemy, enemy_pos) in &mut enemies_q {
            if proj_pos.0.distance_squared(enemy_pos.0) <= hit_radius_sq {
                enemy.hp -= proj.damage;
                commands.entity(proj_entity).despawn();
                if enemy.hp <= 0.0 {
                    commands.entity(enemy_entity).despawn();
                }
                break;
            }
        }
    }
}

fn sync_iso_transforms(mut query: Query<(&WorldPos, &mut Transform)>) {
    for (pos, mut transform) in &mut query {
        let (sx, sy) = world_to_iso(pos.0.x, pos.0.y);
        // Y-sort: lower screen Y renders in front (higher Z).
        let z = 10.0 - sy * 0.001;
        transform.translation = Vec3::new(sx, sy, z);
    }
}

fn camera_follow(
    player_q: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    mut cam_q: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(player_tf) = player_q.get_single() else {
        return;
    };
    let Ok(mut cam_tf) = cam_q.get_single_mut() else {
        return;
    };
    cam_tf.translation.x = player_tf.translation.x;
    cam_tf.translation.y = player_tf.translation.y;
}
