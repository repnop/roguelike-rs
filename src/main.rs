// ============================================================================
// Roguelike-rs - A Shitty Roguelike made in rust with tcod
// @Noah#8080 
// ============================================================================

extern crate tcod;
extern crate rand;

use std::cmp;

use rand::Rng;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap, FovAlgorithm};
use tcod::input::{self, Event, Key, Mouse};

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

// nice stuff
const MAX_ROOM_ITEMS: i32 = 2;
const MAX_INVENTORY_SLOTS: usize = 26;
const INVENTORY_WIDTH: i32 = 50;

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

// lightning shit
const LIGHTNING_RANGE: i32 = 20;
const LIGHTNING_DAMAGE: i32 = 5;

// confuse shit
const CONFUSE_RANGE: i32 = 8;
const CONFUSE_NUM_TURNS: i32 = 10;

// fireball shit
const FIREBALL_RADIUS: i32 = 3;
const FIREBALL_DAMAGE: i32 = 12;

fn main() {

	// set up the window settings
	let root = Root::initializer()
		.font("font/16x.png", FontLayout::AsciiInRow)
		.font_type(FontType::Greyscale)
		.size(SCREEN_WIDTH, SCREEN_HEIGHT)
		.title("rust/tcod test")
		.init();

	// limit the fps
	tcod::system::set_fps(LIMIT_FPS);

	// make all the tcod goodies
	let mut tcod = Tcod {
		root: root,
		con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
		panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
		fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
		mouse: Default::default(),
	};
	
	// initialize the game
	let (mut objects, mut game) = new_game(&mut tcod);

	play_game(&mut objects, &mut game, &mut tcod);
}

// some code to init the game
fn new_game(tcod: &mut Tcod) -> (Vec<Object>, Game) {
	// create player obj
	let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
	player.alive = true;
	player.fighter = Some(Fighter{ max_hp: 30, hp: 30, defense: 2, power: 5, on_death: DeathCallback::Player});

	// obj vec
	let mut objects = vec![player];

	// game struct
	let mut game = Game {
		map: make_map(&mut objects),
		log: vec![],
		inventory: vec![],
	};

	init_fov(&game.map, tcod);

	// welcome message
	game.log.add("what the fuck are you doing playing my shitty roguelike?", colors::RED);

	(objects, game)
}

fn play_game(objects: &mut Vec<Object>, game: &mut Game, tcod: &mut Tcod) {
	// force FOV recompute first time through the loop
	let mut previous_player_position = (-1, -1);

	let mut key = Default::default();

	while !tcod.root.window_closed() {
		match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
			Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
		}

		// draw objects
		let fov_recompute = previous_player_position != (	objects[PLAYER].x, 	objects[PLAYER].y);
		render_all(tcod, game, &objects, fov_recompute);

		tcod.root.flush();

		// clear all objects from their location before moving
		for object in objects.iter_mut() {
			object.clear(&mut tcod.con);
		}

		// handle keys and shit
		previous_player_position = objects[PLAYER].pos();
		let player_action = handle_keys(key, tcod, objects, game);
		if player_action == PlayerAction::Exit {
			break
		}

		// let monsters take their turn
		if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
			for id in 0..objects.len() {
				if objects[id].ai.is_some() {
					ai_take_turn(id, objects, &tcod.fov, game);
				}
			}
		}
	}
}

// init the fov map using the generated map
fn init_fov(map: &Map, tcod: &mut Tcod) {
	for y in 0..MAP_HEIGHT {
		for x in 0..MAP_WIDTH {
			tcod.fov.set(x, y, !map[x as usize][y as usize].block_sight, !map[x as usize][y as usize].blocked);
		}
	}
}

// function to handle keyboard input
fn handle_keys(key: Key, tcod: &mut Tcod, objects: &mut Vec<Object>, game: &mut Game) -> PlayerAction {
	use tcod::input::Key;
	use tcod::input::KeyCode::*;
	use PlayerAction::*;

	let player_alive = objects[PLAYER].alive;
	match (key, player_alive) {
		// togle full screen
		(Key { code: Enter, alt: true, .. }, _) => {
			let current = tcod.root.is_fullscreen();
			tcod.root.set_fullscreen(!current);
			DidntTakeTurn
		}

		// exit with esc key
		(Key { code: Escape, .. }, _) => Exit,

		// basic movement
		(Key { code: Up, .. }, true) => {
			player_move_or_attack(0, -1, objects, game);
			TookTurn
		},
		(Key { code: Down, .. }, true) => {
			player_move_or_attack(0, 1, objects, game);
			TookTurn
		},
		(Key { code: Left, .. }, true) => {
			player_move_or_attack(-1, 0, objects, game);
			TookTurn
		},
		(Key { code: Right, .. }, true) => {
			player_move_or_attack(1, 0, objects, game);
			TookTurn
		},

		(Key { printable: 'g', .. }, true) => {
			//picks up shit from ground
			let item_id = objects.iter().position(|object| {
				object.pos() == objects[PLAYER].pos() && object.item.is_some()
			});
			if let Some(item_id) = item_id {
				pick_item_up(item_id, objects, game);
			}
			DidntTakeTurn
		},

		(Key { printable: 'i', ..}, true) => {
			// shows inventory
			let inventory_index = inventory_menu(&game.inventory, "Press the key next to an item to use it, or any other to cancel.\n", &mut tcod.root);
			if let Some(inventory_index) = inventory_index {
				use_item(tcod, inventory_index, objects, game);
			}
			DidntTakeTurn
		}

		(Key { printable: 'd', ..}, true) => {
			// shows the inventory if an item is selcted drop it
			let inventory_index = inventory_menu(&game.inventory, "Press the key next to an item to drop it, or any other to cancel.\n", &mut tcod.root);
			if let Some(inventory_index) = inventory_index {
				drop_item(inventory_index, objects, game);
			}
			DidntTakeTurn
		}

		(Key { printable: '.', ..}, true) => {
			// wait a turn instead of preforming an action
			game.log.add("You decide to take a moment to think", colors::WHITE);
			TookTurn
		}

		// catch anything else
		_ => DidntTakeTurn,
	}
}

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
	let (x, y) = (mouse.cx as i32, mouse.cy as i32);

	let names = objects.iter().filter(|obj| {obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)}).map(|obj| obj.name.clone()).collect::<Vec<_>>();

	names.join(", ")
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
				rat.ai = Some(Ai::Basic);
				rat
			}
			else {
				let mut kobold = Object::new(x, y, 'k', "kobold", colors::LIGHT_GREEN, true);
				kobold.fighter = Some(Fighter{max_hp: 16, hp: 16, defense: 1, power: 4, on_death: DeathCallback::Monster});
				kobold.ai = Some(Ai::Basic);
				kobold
			};

			monster.alive = true;
			objects.push(monster);
		}

		let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

		for _ in 0..num_items {
			let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
			let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

			if !is_blocked(x, y, map, objects) {
				let dice = rand::random::<f32>();
				let item = if dice < 0.7 {
					// 70% chance for healing pot
					let mut object = Object::new(x, y, '!', "potion of healing", colors::GREEN, false);
					object.item = Some(Item::Heal);
					object
				}
				else if dice < 0.8 {
					// 10% chance to create scroll of lightning
					let mut object = Object::new(x, y, '#', "scroll of lightning", colors::LIGHT_YELLOW, false);
					object.item = Some(Item::Lightning);
					object
				}
				else if dice < 0.9 {
					// 10% chance to create confuse scroll
					let mut object = Object::new(x, y, '#', "scroll of confusion", colors::LIGHT_YELLOW, false);
					object.item = Some(Item::Confuse);
					object
				}
				else {
					// 10% chance to create a fireball scroll
					let mut object = Object::new(x, y, '#', "scroll of fireball", colors::LIGHT_YELLOW, false);
					object.item = Some(Item::Fireball);
					object
				};
				objects.push(item);
			}
		}
	}
}

fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {

	if fov_recompute {
		let player = &objects[PLAYER];
		tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO)
	}
	// render map
	for y in 0..MAP_HEIGHT {
		for x in 0..MAP_WIDTH {
			let visible = tcod.fov.is_in_fov(x, y);
			let wall = game.map[x as usize][y as usize].block_sight;
			let color = match (visible, wall) {
				// outside fov
				(false, true) => COLOR_DARK_WALL,
				(false, false) => COLOR_DARK_GROUND,
				// inside fov
				(true, true) => COLOR_LIGHT_WALL,
				(true, false) => COLOR_LIGHT_GROUND,
			};
			let explored = &mut game.map[x as usize][y as usize].explored;
			if visible {
				*explored = true;
			}
			if *explored {
				tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
			}
			
		}
	}

	// render all objects
	for object in objects {
		if tcod.fov.is_in_fov(object.x, object.y) {
			object.draw(&mut tcod.con);
		}
	}

	let mut to_draw: Vec<_> = objects.iter().filter(|o| tcod.fov.is_in_fov(o.x, o.y)).collect();
	to_draw.sort_by(|o1, o2| {o1.blocks.cmp(&o2.blocks) });

	for object in &to_draw {
		object.draw(&mut tcod.con);
	}

	// prep to render the GUI
	tcod.panel.set_default_background(colors::BLACK);
	tcod.panel.clear();

	// show player stats
	let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
	let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
	render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, colors::LIGHT_RED, colors::DARKER_RED);

	// display names of obj under mouse
	tcod.panel.set_default_foreground(colors::LIGHT_GREY);
	tcod.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left, get_names_under_mouse(tcod.mouse, objects, &tcod.fov));

	// print those msgs
	let mut y = MSG_HEIGHT as i32;
	for &(ref msg, color) in game.log.iter().rev() {
		let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
		y -= msg_height;
		if y < 0 {
			break;
		}
		tcod.panel.set_default_foreground(color);
		tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
	}

	// blit shit
	blit(&tcod.panel, (0, 0), (SCREEN_WIDTH, PANEL_HEIGHT), &mut tcod.root, (0, PANEL_Y), 1.0, 1.0);
	blit(&tcod.con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), &mut tcod.root, (0, 0), 1.0, 1.0);
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

fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
	let (x, y) = objects[id].pos();
	if !is_blocked(x + dx, y + dy, map, objects) {
		objects[id].set_pos(x + dx, y + dy);
	}
}

fn player_move_or_attack(dx: i32, dy: i32, objects: &mut [Object], game: &mut Game) {
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
			player.attack(target, game);
		}
		None => {
			move_by(PLAYER, dx, dy, &game.map, objects);
		}
	}
}

fn pick_item_up(object_id: usize, objects: &mut Vec<Object>, game: &mut Game) {
	if game.inventory.len() >= MAX_INVENTORY_SLOTS {
		game.log.add(format!("Your inventory is full, cannot pick up {}.", objects[object_id].name), colors::RED);
	}
	else {
		let item = objects.swap_remove(object_id);
		game.log.add(format!("You picked up a {}!", item.name), colors::GREEN);
		game.inventory.push(item);
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

fn ai_take_turn(monster_id: usize, objects: &mut [Object], fov_map: &FovMap, game: &mut Game) {
	use Ai::*;
	if let Some(ai) = objects[monster_id].ai.take() {
		let new_ai = match ai {
			Basic => ai_basic(monster_id, objects, fov_map, game),
			Confused{previous_ai, num_turns} => ai_confused(monster_id, objects, game, previous_ai, num_turns),
		};
		objects[monster_id].ai = Some(new_ai);
	}
}

fn ai_basic(monster_id: usize, objects: &mut [Object], fov_map: &FovMap, game: &mut Game) -> Ai {
	// basic monster takes its turn if you can see it, it can see you
	let (monster_x, monster_y) = objects[monster_id].pos();
	if fov_map.is_in_fov(monster_x, monster_y) {
		if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
			// move towards the player if far away
			let (player_x, player_y) = objects[PLAYER].pos();
			move_towards(monster_id, player_x, player_y, &game.map, objects);
		}
		else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
			// close enough to attack
			let (monster, player) = mut_two(monster_id, PLAYER, objects);
			monster.attack(player, game);
		}
	}
	Ai::Basic
}

fn ai_confused(monster_id: usize, objects: &mut [Object], game: &mut Game, previous_ai: Box<Ai>, num_turns: i32) -> Ai {
	if num_turns >= 0 { // still confused
		// move in a random dir and decrease the no. turns remaining
		move_by(monster_id, rand::thread_rng().gen_range(-1, 2), rand::thread_rng().gen_range(-1, 2), &game.map, objects);
		Ai::Confused{previous_ai: previous_ai, num_turns: num_turns - 1}
	}
	else { // restore old Ai
		game.log.add(format!("The {} is no longer confused!", objects[monster_id].name), colors::RED);
		*previous_ai
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

fn player_death(player: &mut Object, game: &mut Game) {
	// end the game
	game.log.add("You died!", colors::DARK_RED);

	player.char = '%';
	player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object, game: &mut Game) {
	game.log.add(format!("{} died", monster.name), colors::DARK_RED);
	monster.char = '%';
	monster.color = colors::DARK_RED;
	monster.blocks = false;
	monster.fighter = None;
	monster.ai = None;
	monster.name = format!("remains of {}", monster.name);
}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
	// make sure the menu isn't too large
	assert!(options.len() <= 26, "Cannot have a menu w/ more than 26 opt");

	// define height of the menu
	let header_height = root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header);
	let height = options.len() as i32 + header_height;

	// create the window size
	let mut window = Offscreen::new(width, height);
	window.set_default_foreground(colors::WHITE);
	window.print_rect_ex(0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, header);

	// iterate through the options and display on window
	for (index, option_text) in options.iter().enumerate() {
		let menu_letter = (b'a' + index as u8) as char;
		let text = format!("({}). {}", menu_letter, option_text.as_ref());
		window.print_ex(0, header_height + index as i32, BackgroundFlag::None, TextAlignment::Left, text);
	}


	// blit menu to root
	let x = SCREEN_WIDTH / 2 - width / 2;
	let y = SCREEN_HEIGHT / 2 - height / 2;
	blit(&mut window, (0, 0), (width, height), root, (x, y), 1.0, 0.7);

	// display the updated root and await keypress
	root.flush();
	let key = root.wait_for_keypress(true);

	// convert ascii code to an index
	if key.printable.is_alphabetic() {
		let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
		if index < options.len() {
			Some(index)
		}
		else {
			None
		}
	}
	else {
		None
	}
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
	// how a menu with each item of inventory as an option
	let options = if inventory.len() == 0 {
		vec!["inventory is empty.".into()]
	}
	else {
		inventory.iter().map(|item| { item.name.clone() }).collect()
	};

	let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

	// if an item was chosen return it
	if inventory.len() > 0 {
		inventory_index
	}
	else {
		None
	}
}

fn closest_monster(max_range: i32, objects: &mut [Object], tcod: &Tcod) -> Option<usize> {
	let mut closest_enemy = None;
	let mut closest_dist = (max_range + 1) as f32;

	for (id, object) in objects.iter().enumerate() {
		if (id != PLAYER) && object.fighter.is_some() && object.ai.is_some() && tcod.fov.is_in_fov(object.x, object.y) {
			let dist = objects[PLAYER].distance_to(object);
			if dist < closest_dist {
				closest_enemy = Some(id);
				closest_dist = dist;
			}
		}
	}
	closest_enemy
}

fn use_item(tcod: &mut Tcod, inventory_id: usize, objects: &mut [Object], game: &mut Game) {
	use Item::*;
	// just call the use function as it's defined
	if let Some(item) = game.inventory[inventory_id].item {
		let on_use: fn(usize, &mut [Object], &mut Game, &mut Tcod) -> UseResult =
		match item {
			Heal => cast_heal,
			Lightning => cast_lightning,
			Confuse => cast_confuse,
			Fireball => cast_fireball,
		};
		match on_use(inventory_id, objects, game, tcod) {
			UseResult::UsedUp => {
				// destroy after use unless it was cancelled for some reason
				game.inventory.remove(inventory_id);
			}
			UseResult::Cancelled => {
				game.log.add("Cancelled", colors::WHITE);
			}
		}
	}
	else {
		game.log.add(format!("The {} cannot be used.", game.inventory[inventory_id].name), colors::WHITE);
	}
}

fn cast_heal(_inventory_id: usize, objects: &mut [Object], game: &mut Game, _tcod: &mut Tcod) -> UseResult {
	// heal the player
	if let Some(fighter) = objects[PLAYER].fighter {
		if fighter.hp == fighter.max_hp {
			game.log.add("You are already at full health.", colors::RED);
			return UseResult::Cancelled;
		}
		game.log.add("your wounds start to feel better!", colors::GREEN);
		objects[PLAYER].heal((fighter.max_hp as f32 * 0.5) as i32);
		return UseResult::UsedUp;
	}
	UseResult::Cancelled
}

fn cast_lightning(_inventory_id: usize, objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) -> UseResult {
	// find closest enemy and damge it
	let monster_id = closest_monster(LIGHTNING_RANGE, objects, tcod);
	if let Some(monster_id) = monster_id {
		// zap that fucker
		game.log.add(format!("A bolt of lightning strikes the {} for {} hp", objects[monster_id].name, LIGHTNING_DAMAGE), colors::LIGHT_BLUE);
		objects[monster_id].take_damage(LIGHTNING_DAMAGE, game);
		UseResult::UsedUp
	}
	else {
		game.log.add("No enemy is close enough to strike.", colors::RED);
		UseResult::Cancelled
	}
}

fn cast_confuse(_inventory_id: usize, objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) -> UseResult {
	game.log.add("left-click an enemy to confuse it, or right-click to cancel.", colors::LIGHT_CYAN);
	let monster_id = target_monster(tcod, objects, game, Some(CONFUSE_RANGE as f32));
	if let Some(monster_id) = monster_id {
		let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
		// replace old Ai with a confused Ai
		objects[monster_id].ai = Some(Ai::Confused {
			previous_ai: Box::new(old_ai),
			num_turns: CONFUSE_NUM_TURNS,
		});
		game.log.add(format!("The {} kinda just went full retard.", objects[monster_id].name), colors::LIGHT_GREEN);
		UseResult::UsedUp
	}
	else {
		game.log.add("No enemy close enough to apply.", colors::RED);
		UseResult::Cancelled
	}
}

fn cast_fireball(_inventory_id: usize, objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) -> UseResult {
	// ask the player for a target tile to throw fireball at
	game.log.add("left-click a target tile for the fireball, or right-click to cancel", colors::LIGHT_CYAN);
	let (x, y) = match target_tile(tcod, objects, game, None) {
		Some(tile_pos) => tile_pos,
		None => return UseResult::Cancelled,
	};
	game.log.add(format!("The fireball explodes, burning everything within {} tiles!", FIREBALL_RADIUS), colors::ORANGE);

	for obj in objects {
		if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
			game.log.add(format!("The {} gets burned for {} hitpoints.", obj.name, FIREBALL_DAMAGE), colors::ORANGE);
			obj.take_damage(FIREBALL_DAMAGE, game);
		}
	}
	UseResult::UsedUp
}

fn target_tile(tcod: &mut Tcod, objects: &[Object], game: &mut Game, max_range: Option<f32>) -> Option<(i32, i32)> {
	use tcod::input::KeyCode::Escape;
	loop {
		// render the screen
		tcod.root.flush();
		let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
		let mut key = None;
		match event {
			Some(Event::Mouse(m)) => tcod.mouse = m,
			Some(Event::Key(k)) => key = Some(k),
			None => {}
		}
		render_all(tcod, game, &objects, false);

		let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);

		// accpet the target if the player clicked in FOV and in range
		let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
		let in_range = max_range.map_or(true, |range| objects[PLAYER].distance(x, y) <= range as f32);
		if tcod.mouse.lbutton_pressed && in_fov && in_range {
			return Some((x, y))
		}

		// exit targeting if esc is pressed
		let escape = key.map_or(false, |k| k.code == Escape);
		if tcod.mouse.rbutton_pressed || escape {
			return None
		}
	}
}

fn target_monster(tcod: &mut Tcod, objects: &[Object], game: &mut Game, max_range: Option<f32>) -> Option<usize> {
	loop {
		match target_tile(tcod, objects, game, max_range) {
			Some((x, y)) => {
				// return the first clicked monster, otherwise continue looping
				for (id, obj) in objects.iter().enumerate() {
					if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
						return Some(id)
					}
				}
			}
			None => return None,
		}
	}
}

fn drop_item(inventory_id: usize, objects: &mut Vec<Object>, game: &mut Game) {
	let mut item = game.inventory.remove(inventory_id);
	item.set_pos(objects[PLAYER].x, objects[PLAYER].y);
	game.log.add(format!("You dropped a {}.", item.name), colors::YELLOW);
	objects.push(item);
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
	item: Option<Item>,
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
			item: None,
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

	pub fn take_damage(&mut self, damage: i32, game: &mut Game) {
		// apply damage if possible
		if let Some(fighter) = self.fighter.as_mut() {
			if damage > 0 {
				fighter.hp -= damage;
			}
		}

		if let Some(fighter) = self.fighter {
			if fighter.hp <= 0 {
				self.alive = false;
				fighter.on_death.callback(self, game);
			}
		}
	}

	pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
		// a simple formula for attack damage
		let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
		if damage > 0 {
			// make the target take some damage
			game.log.add(format!("{} attacks {} for {} hp", self.name, target.name, damage), colors::WHITE);
			target.take_damage(damage, game);
		}
		else {
			game.log.add(format!("{} attacks {} but it glances", self.name, target.name), colors::WHITE);
		}
	}

	pub fn heal(&mut self, amount: i32) {
		if let Some(ref mut fighter) = self.fighter {
			fighter.hp += amount;
			if fighter.hp> fighter.max_hp {
				fighter.hp = fighter.max_hp;
			}
		}
	}

	pub fn distance(&self, x: i32, y: i32) -> f32 {
		(((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
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

#[derive(Clone, Debug, PartialEq)]
enum Ai {
	Basic,
	Confused{previous_ai: Box<Ai>, num_turns: i32},
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DeathCallback {
	Player,
	Monster,
}

impl DeathCallback {
	fn callback(self, object: &mut Object, game: &mut Game) {
		use DeathCallback::*;
		let callback: fn(&mut Object, &mut Game) = match self {
			Player => player_death,
			Monster => monster_death,
		};
		callback(object, game);
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
	Heal,
	Lightning,
	Confuse,
	Fireball,
}

enum UseResult {
	UsedUp,
	Cancelled,
}

struct Tcod {
	root: Root,
	con: Offscreen,
	panel: Offscreen,
	fov: FovMap,
	mouse: Mouse,
}

struct Game {
	map: Map,
	log: Messages,
	inventory: Vec<Object>,
}

trait MessageLog {
	fn add<T: Into<String>>(&mut self, message: T, color: Color);
}

impl MessageLog for Vec<(String, Color)> {
	fn add<T: Into<String>>(&mut self, message: T, color: Color) {
		self.push((message.into(), color));
	}
}