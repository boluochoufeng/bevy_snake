mod fps;

use bevy::{prelude::*, time::common_conditions::on_timer, utils::info};
use fps::FpsPlugin;
use rand::seq::IteratorRandom;
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

const ARENA_WIDTH: i32 = 20;
const ARENA_HEIGHT: i32 = 20;
const SNAKE_HEAD_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);
const SNAKE_SEGMENT_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);
const FOOD_COLOR: Color = Color::srgb(1.0, 0.0, 1.0);

#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct Position {
    x: i32,
    y: i32,
}

fn position_translation(windows: Query<&Window>, mut q: Query<(&Position, &mut Transform)>) {
    let convert = |pos: f32, bound_window: f32, bound_game: f32| -> f32 {
        pos / bound_game * bound_window - bound_window / 2.0 + (bound_window / bound_game) / 2.0
    };

    if let Ok(window) = windows.get_single() {
        for (pos, mut transform) in q.iter_mut() {
            transform.translation = Vec3::new(
                convert(pos.x as f32, window.width(), ARENA_WIDTH as f32),
                convert(pos.y as f32, window.height(), ARENA_HEIGHT as f32),
                0.0,
            );
        }
    }
}

#[derive(Component)]
struct GridSize {
    width: f32,
    height: f32,
}

impl GridSize {
    fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

fn size_scaling(windows: Query<&Window>, mut q: Query<(&GridSize, &mut Transform)>) {
    if let Ok(window) = windows.get_single() {
        for (sprite_size, mut transform) in q.iter_mut() {
            transform.scale = Vec3::new(
                sprite_size.width / ARENA_WIDTH as f32 * window.width(),
                sprite_size.height / ARENA_HEIGHT as f32 * window.height(),
                1.0,
            );
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component)]
struct Food;

fn all_position() -> Vec<Position> {
    let mut positions = Vec::with_capacity((ARENA_WIDTH * ARENA_HEIGHT) as usize);
    for x in 0..ARENA_WIDTH {
        for y in 0..ARENA_HEIGHT {
            positions.push(Position {
                x: x as i32,
                y: y as i32,
            });
        }
    }

    positions
}

fn spawn_food(mut commands: Commands, positions: Query<&Position>) {
    let all_position = all_position();
    let position = all_position
        .iter()
        .filter(|pos| !positions.iter().any(|p| p == *pos))
        .choose(&mut rand::thread_rng());

    if let Some(position) = position {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: FOOD_COLOR,
                    ..default()
                },
                ..default()
            },
            *position,
            GridSize::square(0.8),
            Food,
        ));
    }
}

#[derive(Component)]
struct SnakeHead {
    dir: Direction,
    // snake_movement 每150ms执行一次，snake_movement_input执行了多次
    // 所以在snake_movement之前，可能导致蛇头在动之前反向了
    moved: bool,
}

#[derive(Component)]
struct SnakeSegment;

#[derive(Resource, Default)]
struct SnakeSegments(Vec<Entity>);

impl Deref for SnakeSegments {
    type Target = Vec<Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SnakeSegments {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Resource, Default)]
struct LastTailPosition(Option<Position>);

#[derive(Event)]
struct GrowthEvent;
#[derive(Event)]
struct GameOver;

#[derive(PartialEq, Clone, Copy, Debug)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

fn spawn_snake(mut commands: Commands, mut segments: ResMut<SnakeSegments>) {
    let head_id = commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: SNAKE_HEAD_COLOR,
                    ..default()
                },
                ..default()
            },
            Position { x: 5, y: 5 },
            GridSize::square(0.8),
            SnakeHead {
                dir: Direction::Up,
                moved: false,
            },
        ))
        .id();
    // let segment_id = spawn_snake_segment(commands, Position { x: 5, y: 4 });
    *segments = SnakeSegments(vec![head_id]);
}

fn spawn_snake_segment(mut commands: Commands, position: Position) -> Entity {
    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: SNAKE_SEGMENT_COLOR,
                    ..default()
                },
                ..default()
            },
            position,
            GridSize::square(0.65),
            SnakeSegment,
        ))
        .id()
}

fn snake_movement_input(inputs: Res<ButtonInput<KeyCode>>, mut heads: Query<&mut SnakeHead>) {
    let mut head = heads.single_mut();
    let dir = if inputs.pressed(KeyCode::ArrowUp) {
        Direction::Up
    } else if inputs.pressed(KeyCode::ArrowDown) {
        Direction::Down
    } else if inputs.pressed(KeyCode::ArrowLeft) {
        Direction::Left
    } else if inputs.pressed(KeyCode::ArrowRight) {
        Direction::Right
    } else {
        head.dir
    };

    if head.moved && dir != head.dir && dir != head.dir.opposite() {
        head.dir = dir;
        head.moved = false;
    }
}

fn snake_movement(
    segments: Res<SnakeSegments>,
    mut last_tail_position: ResMut<LastTailPosition>,
    mut heads: Query<(Entity, &mut SnakeHead)>,
    mut positions: Query<&mut Position>,
    mut game_over_writer: EventWriter<GameOver>,
) {
    let segments_positions = segments
        .iter()
        .map(|segment_id| *positions.get(*segment_id).unwrap())
        .collect::<Vec<Position>>();
    let (head_id, mut head) = heads.single_mut();
    let mut head_pos = positions.get_mut(head_id).unwrap();

    match head.dir {
        Direction::Up => head_pos.y += 1,
        Direction::Down => head_pos.y -= 1,
        Direction::Left => head_pos.x -= 1,
        Direction::Right => head_pos.x += 1,
    }
    head.moved = true;

    if head_pos.x < 0
        || head_pos.x >= ARENA_WIDTH as i32
        || head_pos.y < 0
        || head_pos.y >= ARENA_WIDTH as i32
    {
        game_over_writer.send(GameOver);
    }

    if segments_positions.contains(&head_pos) {
        game_over_writer.send(GameOver);
    }

    segments_positions
        .iter()
        .zip(segments.iter().skip(1))
        .for_each(|(pos, segment_id)| {
            *positions.get_mut(*segment_id).unwrap() = *pos;
        });
    *last_tail_position = LastTailPosition(Some(*segments_positions.last().unwrap()));
}

fn snake_eating(
    mut commands: Commands,
    foods: Query<(Entity, &Position), With<Food>>,
    head_position: Query<&Position, With<SnakeHead>>,
    mut growth_wirter: EventWriter<GrowthEvent>,
) {
    let head_position = head_position.single();
    for (food_id, food_position) in foods.iter() {
        if food_position == head_position {
            commands.entity(food_id).despawn();
            growth_wirter.send(GrowthEvent);
        }
    }
}

fn snake_growth(
    commands: Commands,
    last_tail_position: ResMut<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: EventReader<GrowthEvent>,
) {
    if growth_reader.read().next().is_some() {
        let position = last_tail_position.0.unwrap();
        let segment_id = spawn_snake_segment(commands, position);
        segments.push(segment_id);
    }
}

fn game_over(
    mut commands: Commands,
    mut game_over_reader: EventReader<GameOver>,
    segments: ResMut<SnakeSegments>,
    foods: Query<Entity, With<Food>>,
) {
    if game_over_reader.read().next().is_some() {
        foods.iter().for_each(|id| commands.entity(id).despawn());
        segments
            .iter()
            .for_each(|id| commands.entity(*id).despawn());

        info("游戏结束，重新开始");
        spawn_snake(commands, segments);
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.04)))
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: " Snake".into(),
                resolution: (800.0, 800.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FpsPlugin)
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, (size_scaling, position_translation))
        .add_systems(Startup, (spawn_snake, spawn_food))
        .add_systems(
            Update,
            snake_movement.run_if(on_timer(Duration::from_millis(150))),
        )
        .add_systems(Update, snake_movement_input.before(snake_movement))
        .add_systems(Update, game_over.after(snake_movement))
        .add_systems(Update, snake_eating.after(snake_movement))
        .add_systems(Update, snake_growth.after(snake_eating))
        .add_systems(
            Update,
            spawn_food.run_if(on_timer(Duration::from_millis(1500))),
        )
        .add_event::<GrowthEvent>()
        .add_event::<GameOver>()
        .run();
}
