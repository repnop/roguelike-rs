extern crate tcod;
extern crate rand;

use std::cmp;

use rand::Rng;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap, FovAlgorithm};

// a few constants for root memes
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

// FPS limitter
const LIMIT_FPS: i32 = 60;

// map stuff
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 43;

// color stuff
const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b:100 };
const COLOR_LIGHT_WALL: Color =  Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 0, g: 0, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

// room stuffs
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

// create map type
type Map = Vec<Vec<Tile>>;

// msg shiz
type Messages = Vec<(String, Color)>;

// FOV stuff
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

// scary monsters
const MAX_ROOM_MONSTERS: i32 = 3;

// player will always be obj no 1
const PLAYER: usize = 0;

// panel shit
const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

// more panel shit, this time msgs
const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

fn main() {

	// set up the window settings
	let mut root = Root::initializer()
		.font("font/16x.png", FontLayout::AsciiInRow)
		.font_type(FontType::Greyscale)
		.size(SCREEN_WIDTH, SCREEN_HEIGHT)
		.title("rust/tcod test")
		.init();

	// limit the fps
	tcod::system::set_fps(LIMIT_FPS);

	// offscreen console
	let mut con = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);

	// create the player
	let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
	player.alive = true;
	player.fighter = Some(Fighter{ max_hp: 30, hp: 30, defense: 2, power: 5, on_death: DeathCallback::Player});

	// obj vector
	let mut objects = vec![player];

	// gen the map & get starting pos
	let mut map = make_map(&mut objects);

	// FOV stuff
	let mut fov_map = FovMap::new(MAP_WIDTH, MAP_HEIGHT);
	for y in 0..MAP_HEIGHT {
		for x in 0..MAP_WIDTH {
			fov_map.set(x, y, !map[x as usize][y as usize].block_sight, !map[x as usize][y as usize].blocked);
		}
	}

	// force FOV "recompute" first time through the loop
	let mut previous_player_position = (-1, -1);

	// gui time bb
	let mut panel = Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT);

	// message vector
	let mut messages = vec![];

	// some welcome message
	message(&mut messages, "yo wus poppin b? u finna die in this dungeon", colors::RED);

	// main game loop
	while !root.window_closed() {
		// draw objects
		let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
		render_all(&mut root, &mut con, &mut panel, &messages, &objects, &mut map, &mut fov_map, fov_recompute);
		
		root.flush();

		// clear all objects from their location before moving
		for object in &objects {
			object.clear(&mut con);
		}

		// handle keys and shit
		previous_player_position = objects[PLAYER].pos();
		let player_action = handle_keys(&mut root, &map, &mut objects);
		if player_action == PlayerAction::Exit {
			break
		}

		// let monsters take their turn
		if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
			for id in 0..objects.len() {
				if objects[id].ai.is_some() {
					ai_take_turn(id, &map, &mut objects, &fov_map);
				}
			}
		}
	}
}

// function to handle keyboard input
fn handle_keys(root: &mut Root, map: &Map, objects: &mut [Object]) -> PlayerAction {
	use tcod::input::Key;
	use tcod::input::KeyCode::*;
	use PlayerAction::*;

	let key = root.wait_for_keypress(true);
	let player_alive = objects[PLAYER].alive;
	match (key, player_alive) {
		// togle full screen
		(Key { code: Enter, alt: true, .. }, _) => {
			let current = root.is_fullscreen();
			root.set_fullscreen(!current);
			DidntTakeTurn
		}

		// exit with esc key
		(Key { code: Escape, .. }, _) => Exit,

		// basic movement
		(Key { code: Up, .. }, true) => {
			player_move_or_attack(0, -1, map, objects);
			TookTurn
		},
		(Key { code: Down, .. }, true) => {
			player_move_or_attack(0, 1, map, objects);
			TookTurn
		},
		(Key { code: Left, .. }, true) => {
			player_move_or_attack(-1, 0, map, objects);
			TookTurn
		},
		(Key { code: Right, .. }, true) => {
			player_move_or_attack(1, 0, map, objects);
			TookTurn
		},

		_ => DidntTakeTurn,
	}
}

fn make_map(objects: &mut Vec<Object>) -> Map {
	// fill the map with blocked tiles
	let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

	let mut rooms = vec![];

	for _ in 0..MAX_ROOMS {
		let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
		let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);

		let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
		let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

		let new_room = Rect::new(x, y, w, h);

		// iterate through
		let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room));

		if !failed {
			create_room(new_room, &mut map);

			place_objects(new_room, &map, objects);

			let (new_x, new_y) = new_room.center();

			if rooms.is_empty() {
				objects[PLAYER].set_pos(new_x, new_y);
			}

			else {
				let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

				if rand::random() {
					create_h_tunnel(prev_x, new_x, prev_y, &mut map);
					create_v_tunnel(prev_y, new_y, new_x, &mut map);
				}
				else {
					create_v_tunnel(prev_y, new_y, prev_x, &mut map);
					create_h_tunnel(prev_x, new_x, new_y, &mut map);
				}
			}

			rooms.push(new_room);
		}
	}

	map
}

fn create_room(room: Rect, map: &mut Map) {
	for x in (room.x1 + 1)..room.x2 {
		for y in (room.y1 + 1)..room.y2 {
			map[x as usize][y as usize] = Tile::empty();
		}
	}
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
	for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
		map[x as usize][y as usize] = Tile::empty();
	}
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
	for y in cmp::min(y1, y2)..(cmp::max(y1, y2,) + 1) {
		map[x as usize][y as usize] = Tile::empty();
	}
}

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
	let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

	for _ in 0..num_monsters {
		let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
		let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

		if !is_blocked(x, y, map, objects) {
			let mut monster = if rand::random::<f32>() < 0.8 {
				let mut rat = Object::new(x, y, 'r', "rat", colors::YELLOW, true);
				rat.fighter = Some(Fighter{max_hp: 10, hp: 10, defense: 0, power: 3, on_death: DeathCallback::Monster});
				rat.ai = Some(Ai);
				rat
			}
			else {
				let mut kobold = Object::new(x, y, 'k', "kobold", colors::LIGHT_GREEN, true);
				kobold.fighter = Some(Fighter{max_hp: 16, hp: 16, defense: 1, power: 4, on_death: DeathCallback::Monster});
				kobold.ai = Some(Ai);
				kobold
			};

			monster.alive = true;
			objects.push(monster);
		}
	}
}

fn render_all(root: &mut Root, con: &mut Offscreen, panel: &mut Offscreen, messages: &Messages, objects: &[Object], map: &mut Map, fov_map: &mut FovMap, fov_recompute: bool) {

	if fov_recompute {
		let player = &objects[PLAYER];
		fov_map.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO)
	}
	// render map
	for y in 0..MAP_HEIGHT {
		for x in 0..MAP_WIDTH {
			let visible = fov_map.is_in_fov(x, y);
			let wall = map[x as usize][y as usize].block_sight;
			let color = match (visible, wall) {
				// outside fov
				(false, true) => COLOR_DARK_WALL,
				(false, false) => COLOR_DARK_GROUND,
				// inside fov
				(true, true) => COLOR_LIGHT_WALL,
				(true, false) => COLOR_LIGHT_GROUND,
			};
			let explored = &mut map[x as usize][y as usize].explored;
			if visible {
				*explored = true;
			}
			if *explored {
				con.set_char_background(x, y, color, BackgroundFlag::Set);
			}
			
		}
	}

	// render all objects
	for object in objects {
		if fov_map.is_in_fov(object.x, object.y) {
			object.draw(con);
		}
	}

	let mut to_draw: Vec<_> = objects.iter().filter(|o| fov_map.is_in_fov(o.x, o.y)).collect();
	to_draw.sort_by(|o1, o2| {o1.blocks.cmp(&o2.blocks) });

	for object in &to_draw {
		object.draw(con);
	}

	// prep to render the GUI
	panel.set_default_background(colors::BLACK);
	panel.clear();

	// show player stats
	let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
	let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
	render_bar(panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, colors::LIGHT_RED, colors::DARKER_RED);

	// print those msgs
	let mut y = MSG_HEIGHT as i32;
	for &(ref msg, color) in messages.iter().rev() {
		let msg_height = panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
		y -= msg_height;
		if y < 0 {
			break;
		}
		panel.set_default_foreground(color);
		panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
	}

	// blit shit
	blit(panel, (0, 0), (SCREEN_WIDTH, PANEL_HEIGHT), root, (0, PANEL_Y), 1.0, 1.0);
	blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0);
}

fn render_bar(panel: &mut Offscreen, x: i32, y: i32, total_width: i32, name: &str, value: i32, maximum: i32, bar_color: Color, back_color: Color) {
	let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

	// render bg first
	panel.set_default_background(back_color);
	panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

	// now render bar ontop
	panel.set_default_background(bar_color);
	if bar_width > 0 {
		panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
	}

	panel.set_default_foreground(colors::WHITE);
	panel.print_ex(x + total_width / 2, y, BackgroundFlag::None, TextAlignment::Center, &format!("{}: {}/{}", name, value, maximum));
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
	// test if map tile
	if map[x as usize][y as usize].blocked {
		return true;
	}
	// check if any blocking objects
	objects.iter().any(|object| {
		object.blocks && object.pos() == (x, y)
	})
}

// move by a given amount
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
	let (x, y) = objects[id].pos();
	if !is_blocked(x + dx, y + dy, map, objects) {
		objects[id].set_pos(x + dx, y + dy);
	}
}

fn player_move_or_attack(dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
	// coords the player is moving / attacking to
	let x = objects[PLAYER].x + dx;
	let y = objects[PLAYER].y + dy;

	// try to find an attackable object there
	let target_id = objects.iter().position(|object| {
		object.fighter.is_some() && object.pos() == (x, y)
	});

	// attack target if found otherwise move
	match target_id {
		Some(target_id) => {
			let (player, target) = mut_two(PLAYER, target_id, objects);
			player.attack(target);
		}
		None => {
			move_by(PLAYER, dx, dy, map, objects);
		}
	}
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
	// vector from this obj to targ + dist
	let dx = target_x - objects[id].x;
	let dy = target_y - objects[id].y;
	let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

	// normalize it to lenght 1 (preserving direction) then round it
	// and convert to integer so the movement is restricted to the map grid
	let dx = (dx as f32 / distance).round() as i32;
	let dy = (dy as f32 / distance).round() as i32;
	move_by(id, dx, dy, map, objects);
}

fn ai_take_turn(monster_id: usize, map: &Map, objects: &mut [Object], fov_map: &FovMap) {
	// a basic monster takes it's turn, if you can see it, it can see you
	let (monster_x, monster_y) = objects[monster_id].pos();
	if fov_map.is_in_fov(monster_x, monster_y) {
		if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
			// move towards the player if far away
			let (player_x, player_y) = objects[PLAYER].pos();
			move_towards(monster_id, player_x, player_y, map, objects);
		}
		else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
			// close enough to attack
			let (monster, player) = mut_two(monster_id, PLAYER, objects);
			monster.attack(player);
		}
	}
}

fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
	assert!(first_index != second_index);
    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    }
    else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}

fn player_death(player: &mut Object) {
	// end the game
	println!("You died!");

	player.char = '%';
	player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object) {
	println!("{} died", monster.name);
	monster.char = '%';
	monster.color = colors::DARK_RED;
	monster.blocks = false;
	monster.fighter = None;
	monster.ai = None;
	monster.name = format!("remains of {}", monster.name);
}

fn message<T: Into<String>>(messages: &mut Messages, message: T, color: Color) {
	// if buffer is full remove first message to make room for new ones
	if messages.len() == MSG_HEIGHT {
		messages.remove(0);
	}
	// add the new line as a tuple with text / color
	messages.push((message.into(), color));
}

// object stuffs
#[derive(Debug)]
struct Object {
	x: i32,
	y: i32,
	char: char,
	color: Color,
	name: String,
	blocks: bool,
	alive: bool,
	fighter: Option<Fighter>,
	ai: Option<Ai>,
}

// object functions
impl Object {
	// create a new object
	pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
		Object {
			x: x,
			y: y,
			char: char,
			color: color,
			name: name.into(),
			blocks: blocks,
			alive: false,
			fighter: None,
			ai: None,
		}
	}

	// set color then draw the char at given location
	pub fn draw(&self, con: &mut Console) {
		con.set_default_foreground(self.color);
		con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
	}

	// Erase the character
	pub fn clear(&self, con: &mut Console) {
		con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
	}

	pub fn pos(&self) -> (i32, i32) {
		(self.x, self.y)
	}

	pub fn set_pos(&mut self, x: i32, y: i32) {
		self.x = x;
		self.y = y;
	}

	pub fn distance_to(&self, other: &Object) -> f32 {
		let dx = other.x - self.x;
		let dy = other.y - self.y;
		((dx.pow(2) + dy.pow(2)) as f32).sqrt()
	}

	pub fn take_damage(&mut self, damage: i32) {
		// apply damage if possible
		if let Some(fighter) = self.fighter.as_mut() {
			if damage > 0 {
				fighter.hp -= damage;
			}
		}

		if let Some(fighter) = self.fighter {
			if fighter.hp <= 0 {
				self.alive = false;
				fighter.on_death.callback(self);
			}
		}
	}

	pub fn attack(&mut self, target: &mut Object) {
		// a simple formula for attack damage
		let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
		if damage > 0 {
			// make the target take some damage
			println!("{} attacks {} for {} hp", self.name, target.name, damage);
			target.take_damage(damage);
		}
		else {
			println!("{} attacks {} but it glances", self.name, target.name);
		}
	}
}

#[derive(Clone, Copy, Debug)]
struct Tile {
	blocked: bool,
	block_sight: bool,
	explored: bool,
}

impl Tile {
	// empty space tile
	pub fn empty() -> Self {
		Tile{blocked: false, block_sight: false, explored: false}
	}

	// wall tile
	pub fn wall() -> Self {
		Tile{blocked: true, block_sight: true, explored: false}
	}
}

#[derive(Clone, Copy, Debug)]
struct Rect {
	x1: i32,
	y1: i32,
	x2: i32,
	y2: i32,
}

impl Rect {
	pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
		Rect { x1: x, y1: y, x2: x + w, y2: y + h}
	}

	pub fn center(&self) -> (i32, i32) {
		let center_x = (self.x1 + self.x2) / 2;
		let center_y = (self.y1 + self.y2) / 2;
		(center_x, center_y)
	}

	pub fn intersects_with(&self, other: &Rect) -> bool {
		// returns true if current rect intersects with another
		(self.x1 <= other.x2) && (self.x2 >= other.x1) && (self.y1 <= other.y2) && (self.y2 >= other.y1)
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
	TookTurn,
	DidntTakeTurn,
	Exit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Fighter {
	max_hp: i32,
	hp: i32,
	defense: i32,
	power: i32,
	on_death: DeathCallback,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Ai;

#[derive(Clone, Copy, Debug, PartialEq)]
enum DeathCallback {
	Player,
	Monster,
}

impl DeathCallback {
	fn callback(self, object: &mut Object) {
		use DeathCallback::*;
		let callback: fn(&mut Object) = match self {
			Player => player_death,
			Monster => monster_death,
		};
		callback(object);
	}
}