use bevy::prelude::*;
use rand::prelude::*;
use std::time::Duration;

/*
=========================================================================
RUST SURVIVOR: LIBRARY MODULE
=========================================================================
*/

const AREA_SIZE: f32 = 2400.0;
const BOUND: f32 = AREA_SIZE / 2.0;
const GRID_STEP: f32 = 120.0;

// Это главная точка входа, которую вызовет main.rs или WASM
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rust Survivor 0.18.1".into(),
                #[cfg(target_arch = "wasm32")]
                canvas: Some("#bevy-canvas".into()),
                #[cfg(target_arch = "wasm32")]
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((CorePlugin, PlayerPlugin, EnemyPlugin, CombatPlugin))
        .run();
}

// ==========================================
// МОДУЛЬ: CORE
// ==========================================
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_camera, spawn_grid))
           .add_systems(Update, (world_wrap_system, update_grid_position));
    }
}

#[derive(Component)] pub struct Health(pub f32);
#[derive(Component)] pub struct Collider { pub radius: f32 }
#[derive(Component)] pub struct MainCamera;
#[derive(Component)] pub struct GridParent;

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

fn spawn_grid(mut commands: Commands) {
    commands.spawn((
        GridParent,
        Transform::default(),
        Visibility::default(),
    )).with_children(|parent| {
        for i in -12..12 {
            for j in -12..12 {
                parent.spawn((
                    Sprite {
                        color: Color::srgb(0.1, 0.1, 0.15),
                        custom_size: Some(Vec2::new(2.0, 2.0)),
                        ..default()
                    },
                    Transform::from_xyz(i as f32 * GRID_STEP, j as f32 * GRID_STEP, -1.0),
                ));
            }
        }
    });
}

fn update_grid_position(
    player_q: Query<&Transform, (With<Player>, Without<GridParent>)>,
    mut grid_q: Query<&mut Transform, With<GridParent>>,
) {
    let Ok(p_t) = player_q.single() else { return };
    let Ok(mut g_t) = grid_q.single_mut() else { return };
    
    g_t.translation.x = (p_t.translation.x / GRID_STEP).round() * GRID_STEP;
    g_t.translation.y = (p_t.translation.y / GRID_STEP).round() * GRID_STEP;
}

fn world_wrap_system(mut query: Query<&mut Transform, With<Collider>>) {
    for mut transform in &mut query {
        let mut pos = transform.translation;
        if pos.x > BOUND { pos.x -= AREA_SIZE; }
        else if pos.x < -BOUND { pos.x += AREA_SIZE; }
        if pos.y > BOUND { pos.y -= AREA_SIZE; }
        else if pos.y < -BOUND { pos.y += AREA_SIZE; }
        transform.translation = pos;
    }
}

// ==========================================
// МОДУЛЬ: PLAYER
// ==========================================
pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
           .add_systems(Update, (player_movement, player_aiming, camera_follow));
    }
}

#[derive(Component)] pub struct Player { pub speed: f32, pub facing: Vec2 }

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Sprite { color: Color::srgb(0.0, 0.9, 0.4), custom_size: Some(Vec2::splat(32.0)), ..default() },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Player { speed: 400.0, facing: Vec2::X },
        Health(100.0),
        Collider { radius: 16.0 },
    ));
}

fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut q: Query<(&Player, &mut Transform)>,
) {
    let Ok((player, mut trans)) = q.single_mut() else { return };
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) { dir.x += 1.0; }

    if dir != Vec2::ZERO {
        trans.translation += dir.normalize().extend(0.0) * player.speed * time.delta_secs();
    }
}

fn player_aiming(
    win_q: Query<&Window>,
    cam_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut p_q: Query<(&Transform, &mut Player)>,
) {
    let Ok(window) = win_q.single() else { return };
    let Ok((cam, cam_gt)) = cam_q.single() else { return };
    let Ok((p_t, mut player)) = p_q.single_mut() else { return };

    if let Some(cursor_pos) = window.cursor_position()
        .and_then(|c| cam.viewport_to_world_2d(cam_gt, c).ok()) {
        let diff = cursor_pos - p_t.translation.truncate();
        if diff != Vec2::ZERO {
            player.facing = diff.normalize();
        }
    }
}

fn camera_follow(
    player_q: Query<&Transform, With<Player>>,
    mut cam_q: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    let Ok(p_t) = player_q.single() else { return };
    let Ok(mut c_t) = cam_q.single_mut() else { return };
    c_t.translation = p_t.translation.truncate().extend(10.0);
}

// ==========================================
// МОДУЛЬ: ENEMY
// ==========================================
pub struct EnemyPlugin;
impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnemySpawner { timer: Timer::new(Duration::from_secs(2), TimerMode::Repeating) })
           .add_systems(Update, (spawn_enemies, enemy_behavior, resolve_enemy_collisions));
    }
}

#[derive(Resource)] pub struct EnemySpawner { pub timer: Timer }
#[derive(Component)] pub struct Enemy { pub speed: f32, pub attack_timer: Timer }

fn spawn_enemies(
    time: Res<Time>,
    mut spawner: ResMut<EnemySpawner>,
    mut cmd: Commands,
    p_q: Query<&Transform, With<Player>>
) {
    spawner.timer.tick(time.delta());
    if spawner.timer.just_finished() {
        let Ok(p_t) = p_q.single() else { return };
        let mut rng = rand::rng();
        for _ in 0..3 {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let spawn_pos = p_t.translation.truncate() + Vec2::new(angle.cos(), angle.sin()) * 600.0;
            
            cmd.spawn((
                Sprite { color: Color::srgb(1.0, 0.2, 0.2), custom_size: Some(Vec2::splat(24.0)), ..default() },
                Transform::from_translation(spawn_pos.extend(0.0)),
                Enemy { speed: 150.0, attack_timer: Timer::new(Duration::from_secs(1), TimerMode::Repeating) },
                Health(30.0),
                Collider { radius: 12.0 },
            ));
        }
    }
}

fn enemy_behavior(
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    mut enemy_q: Query<(&mut Transform, &mut Enemy, &Collider), Without<Player>>,
    mut p_health_q: Query<&mut Health, With<Player>>,
) {
    let Ok(p_t) = player_q.single() else { return };
    let p_pos = p_t.translation;
    let mut total_damage = 0.0;

    for (mut e_t, mut enemy, e_c) in &mut enemy_q {
        let diff = p_pos - e_t.translation;
        let dist = diff.length();
        
        if dist > (e_c.radius + 10.0) {
            e_t.translation += diff.normalize() * enemy.speed * time.delta_secs();
        } else {
            enemy.attack_timer.tick(time.delta());
            if enemy.attack_timer.just_finished() {
                total_damage += 10.0;
            }
        }
    }

    if total_damage > 0.0 {
        if let Ok(mut h) = p_health_q.single_mut() {
            h.0 -= total_damage;
        }
    }
}

fn resolve_enemy_collisions(mut enemy_q: Query<(&mut Transform, &Collider), With<Enemy>>) {
    let mut combinations = enemy_q.iter_combinations_mut();
    while let Some([(mut t1, c1), (mut t2, c2)]) = combinations.fetch_next() {
        let dist = t1.translation.distance(t2.translation);
        let min_dist = c1.radius + c2.radius;
        if dist < min_dist && dist > 0.1 {
            let push = (t1.translation - t2.translation).normalize() * (min_dist - dist) * 0.5;
            t1.translation += push;
            t2.translation -= push;
        }
    }
}

// ==========================================
// МОДУЛЬ: COMBAT
// ==========================================
pub struct CombatPlugin;
impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (player_shoot, move_projectiles, bullet_collision_system));
    }
}

#[derive(Component)] pub struct Bullet { pub dir: Vec2, pub lifetime: Timer }

fn player_shoot(
    mut cmd: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    p_q: Query<(&Transform, &Player)>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let Ok((p_t, player)) = p_q.single() else { return };
        cmd.spawn((
            Sprite { color: Color::srgb(1.0, 1.0, 0.0), custom_size: Some(Vec2::splat(8.0)), ..default() },
            Transform::from_translation(p_t.translation),
            Bullet { dir: player.facing, lifetime: Timer::new(Duration::from_secs(2), TimerMode::Once) },
            Collider { radius: 4.0 },
        ));
    }
}

fn move_projectiles(time: Res<Time>, mut cmd: Commands, mut q: Query<(Entity, &mut Transform, &mut Bullet)>) {
    for (entity, mut trans, mut bullet) in &mut q {
        bullet.lifetime.tick(time.delta());
        if bullet.lifetime.just_finished() {
            if let Ok(mut e_cmd) = cmd.get_entity(entity) { e_cmd.despawn(); }
            continue;
        }
        trans.translation += bullet.dir.extend(0.0) * 800.0 * time.delta_secs();
    }
}

fn bullet_collision_system(
    mut cmd: Commands,
    bullets_q: Query<(Entity, &Transform, &Collider), With<Bullet>>,
    mut enemies_q: Query<(Entity, &Transform, &Collider, &mut Health), With<Enemy>>,
) {
    for (b_entity, b_t, b_c) in &bullets_q {
        for (e_entity, e_t, e_c, mut health) in &mut enemies_q {
            if b_t.translation.distance(e_t.translation) < (b_c.radius + e_c.radius) {
                health.0 -= 15.0;
                if let Ok(mut b_cmd) = cmd.get_entity(b_entity) { b_cmd.despawn(); }
                if health.0 <= 0.0 {
                    if let Ok(mut e_cmd) = cmd.get_entity(e_entity) { e_cmd.despawn(); }
                }
                break;
            }
        }
    }
}