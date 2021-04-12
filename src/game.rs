use bevy::prelude::*;
use rand::prelude::random;

pub struct Food;

pub struct SnakeHead {
  pub input_direction: Direction,
  pub movement_direction: Direction,
}

pub struct SnakeSegment;

#[derive(Default)]
pub struct SnakeSegments(Vec<Entity>);

pub struct Materials {
  pub head_material: Handle<ColorMaterial>,
  pub segment_material: Handle<ColorMaterial>,
  pub food_material: Handle<ColorMaterial>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Position {
  x: i32,
  y: i32,
}

pub struct Size {
  width: f32,
  height: f32,
}

impl Size {
  pub fn square(x: f32) -> Self {
    Self {
      width: x,
      height: x,
    }
  }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
  Left,
  Up,
  Right,
  Down,
}

impl Direction {
  pub fn opposite(&self) -> Self {
    match self {
      Self::Left => Self::Right,
      Self::Up => Self::Down,
      Self::Right => Self::Left,
      Self::Down => Self::Up,
    }
  }
}

#[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SnakeMovement {
  Input,
  Movement,
  Eating,
  Growth,
}

#[derive(Default)]
pub struct LastTailPosition(Option<Position>);

pub struct GrowthEvent;
pub struct GameOverEvent;

pub const ARENA_WIDTH: u32 = 10;
pub const ARENA_HEIGHT: u32 = 10;

pub fn spawn_segment(
  mut commands: Commands,
  material: Handle<ColorMaterial>,
  position: Position
) -> Entity {
  commands
    .spawn_bundle(SpriteBundle { material, ..Default::default() })
    .insert(SnakeSegment)
    .insert(position)
    .insert(Size::square(0.65))
    .id()
}

pub fn spawn_snake(
  mut commands: Commands,
  mut segments: ResMut<SnakeSegments>,
  materials: Res<Materials>
) {
  segments.0 = vec![
    commands
      .spawn_bundle(SpriteBundle {
        material: materials.head_material.clone(),
        sprite: Sprite::new(Vec2::new(10.0, 10.0)),
        ..Default::default()
      })
      .insert(SnakeHead {
        input_direction: Direction::Up,
        movement_direction: Direction::Up
      })
      .insert(SnakeSegment)
      .insert(Position { x: 3, y: 3 })
      .insert(Size::square(0.8))
      .id(),
    spawn_segment(
      commands,
      materials.segment_material.clone(),
      Position { x: 3, y: 2 },
    ),
  ]
}

pub fn snake_movement_input(input: Res<Input<KeyCode>>, mut heads: Query<&mut SnakeHead>) {
  if let Some(mut head) = heads.iter_mut().next() {
    let new_direction = {
      if input.pressed(KeyCode::Left) { Direction::Left }
      else if input.pressed(KeyCode::Right) { Direction::Right}
      else if input.pressed(KeyCode::Up) { Direction::Up }
      else if input.pressed(KeyCode::Down) { Direction::Down }
      else { head.input_direction }
    };

    if new_direction != head.movement_direction.opposite() {
      head.input_direction = new_direction;
    }
  }
}

pub fn snake_movement(
  segments: Res<SnakeSegments>,
  mut last_tail_position: ResMut<LastTailPosition>,
  mut heads: Query<(Entity, &mut SnakeHead)>,
  mut positions: Query<&mut Position>,
  mut game_over_writer: EventWriter<GameOverEvent>,
) {
  if let Some((head_entity, mut head)) = heads.iter_mut().next() {
    let segment_positions = segments.0
      .iter()
      .map(|&entity| *positions.get_mut(entity).unwrap())
      .collect::<Vec<Position>>();
    let mut head_position = positions.get_mut(head_entity).unwrap();
    match head.input_direction {
      Direction::Left => { head_position.x -= 1; },
      Direction::Up => { head_position.y += 1; },
      Direction::Right => { head_position.x += 1; },
      Direction::Down => { head_position.y -= 1; },
    }
    head.movement_direction = head.input_direction;
    if head_position.x < 0 ||
        head_position.x as u32 >= ARENA_WIDTH ||
        head_position.y < 0 ||
        head_position.y as u32 >= ARENA_HEIGHT {
      game_over_writer.send(GameOverEvent);
    }
    if segment_positions.contains(&head_position) {
      game_over_writer.send(GameOverEvent);
    }
    segment_positions
      .iter()
      .zip(segments.0.iter().skip(1))
      .for_each(|(&position, &segment)| {
        *positions.get_mut(segment).unwrap() = position;
      });
    last_tail_position.0 = segment_positions.last().copied();
  }
}

pub fn snake_eating(
  mut commands: Commands,
  mut growth_writer: EventWriter<GrowthEvent>,
  food_positions: Query<(&Position, Entity), With<Food>>,
  head_positions: Query<&Position, With<SnakeHead>>,
) {
  if let Some(head_position) = head_positions.iter().next() {
    for (food_position, food_entity) in food_positions.iter() {
      if food_position == head_position {
        commands.entity(food_entity).despawn();
        growth_writer.send(GrowthEvent);
      }
    }
  }
}

pub fn snake_growth(
  commands: Commands,
  last_tail_position: Res<LastTailPosition>,
  mut segments: ResMut<SnakeSegments>,
  mut growth_reader: EventReader<GrowthEvent>,
  materials: Res<Materials>,
) {
  if growth_reader.iter().next().is_some() {
    segments.0.push(spawn_segment(
      commands,
      materials.segment_material.clone(),
      last_tail_position.0.unwrap(),
    ));
  }
}

pub fn game_over(
  mut commands: Commands,
  mut reader: EventReader<GameOverEvent>,
  materials: Res<Materials>,
  segments_res: ResMut<SnakeSegments>,
  food: Query<Entity, With<Food>>,
  segments: Query<Entity, With<SnakeSegment>>,
) {
  if reader.iter().next().is_some() {
    for entity in food.iter().chain(segments.iter()) {
      commands.entity(entity).despawn();
    }
    spawn_snake(commands, segments_res, materials);
  }
}

pub fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Sprite)>) {
  let window = windows.get_primary().expect("Couldn't find primary window!");
  for (sprite_size, mut sprite) in q.iter_mut() {
    sprite.size = Vec2::new(
      window.width() / (ARENA_WIDTH as f32) * sprite_size.width,
      window.height() / (ARENA_HEIGHT as f32) * sprite_size.height,
    );
  }
}

pub fn position_translation(windows: Res<Windows>, mut q: Query<(&Position, &mut Transform)>) {
  let window = windows.get_primary().expect("Couldn't find primary window!");

  fn convert_dimension(dimension: f32, window_dimension: f32, arena_dimension: f32) -> f32 {
    let tile_dimension = window_dimension / arena_dimension;
    dimension * tile_dimension - window_dimension / 2.0 + tile_dimension / 2.0
  }

  for (pos, mut transform) in q.iter_mut() {
    transform.translation = Vec3::new(
      convert_dimension(pos.x as f32, window.width(), ARENA_WIDTH as f32),
      convert_dimension(pos.y as f32, window.height(), ARENA_HEIGHT as f32),
      0.0,
    );
  }
}

pub fn food_spawner(
  mut commands: Commands,
  materials: Res<Materials>,
  food_entities: Query<Entity, With<Food>>,
  segment_entities: Query<Entity, With<SnakeSegment>>,
  positions: Query<&Position>,
) {
  let position = loop {
    let position = Position {
      x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
      y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
    };
    let taken_positions = food_entities
      .iter()
      .chain(segment_entities.iter())
      .map(|entity| *positions.get(entity).unwrap())
      .collect::<Vec<Position>>();
    if !taken_positions.contains(&position) { break position; }
  };

  commands
    .spawn_bundle(SpriteBundle {
      material: materials.food_material.clone(),
      ..Default::default()
    })
    .insert(Food)
    .insert(position)
    .insert(Size::square(0.8));
}
