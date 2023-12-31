use std::net::TcpStream;
use std::io::{Read, Write};
use std::{thread, sync::{Arc, Mutex, RwLock}};
use rand::prelude::*;
use std::time::Duration;


use tetra::{graphics::{mesh::{Mesh, ShapeStyle, GeometryBuilder}, self, text::{Font, Text}, Color, Rectangle},
input::{self, Key}, Context, ContextBuilder, State};
use tetra::math::{Vec2, Rect};


#[derive(Clone, Debug)]
struct GlobalPlayerDetails {
    id: usize,
    name: String,
    position: [f32; 2],
    recent_bullet_id: String
   // message: Vec<String>
}
#[derive(Clone, Debug)]
struct LocalPlayerDetails {
    name: String,
    position: [f32; 2],
    recent_bullet_position_and_direction: [f32; 3], 
    message: String,
    // Used for other clients to determine when a new bullet has come compared to the previous 'new_bullet_id'
    new_bullet_id: usize
}

#[derive(Clone, Debug)]
struct PlayersMessage {
    id: usize,
    msg: String
}

#[derive(Clone, Debug)]
struct PlayersBullet {
    players_bullet_id: String,
    players_name: String,
    rect: Rectangle,
    direction: f32,
    speed: f32
}

/* GLOBALS & CONSTS */
static mut LOCAL_DETAILS:LocalPlayerDetails = LocalPlayerDetails {name: (String::new()), position: ([0.0, 0.0]), recent_bullet_position_and_direction: ([0.0, 0.0, 0.0]), message: (String::new()), new_bullet_id: (0)};
static mut PLAYERS_DETAILS:Vec<GlobalPlayerDetails> = Vec::new();
static mut PLAYERS_MESSAGES:Vec<PlayersMessage> = Vec::new();
static mut CHAT_LOG:Vec<String> = Vec::new();
static mut PREVIOUS_PLAYER_FOUND:String = String::new();

//static mut ACTIVE_BULLETS:Mutex<Vec<PlayersBullet>> = Mutex::new(Vec::new());
static mut NEW_BULLETS_BUF:Mutex<Vec<PlayersBullet>> = Mutex::new(Vec::new());

const UNSET_BULLET:[f32; 3] = [f32::MAX; 3];

const MOVEMENT_SPEED: f32 = 2.0;
const SCREEN_SIZE:[i32; 2] = [900, 720]; 
const TEXT_SIZE:f32 = 17.0;

const MAX_PLAYERS:usize = 20;

static mut LOCAL_DESIRES_CONNECTED:bool = true;
static mut LOCAL_IS_HIT: bool = false;

const PLAYER_WIDTH:f32 = 20.0;
const BULLET_WIDTH:f32 = 10.0;
static mut MAPS_RECTANGLE:Rectangle = Rectangle::new(0.0, 0.0, 0.0, 0.0);

/* TETRA */
struct GameState {
    // Meshes
    mouse_shape: Mesh,
    map_shape: Mesh,
    map_rect: Rectangle<f32>,
    player_shape: Mesh,
    player_rect: Rectangle<f32>,  
    chat_box_shape: Mesh, 
    chat_box_rect: Rectangle<f32>,
    chat_box_line_shape: Mesh,
    bullet_shape: Mesh,
    bullet_rect: Rectangle<f32>,
//    current_bullet_position: [f32; 2],

    current_active_bullets: Vec<PlayersBullet>,

    text: Text,
    chat_mode: bool,
    
    local_player_position: [f32; 2],  
    local_player_message: String,

    scroll: [f32; 2] 
}
impl GameState {
    fn new(ctx: &mut Context) -> tetra::Result<GameState> {
        let maps_rectangle_stack = Rectangle::new(100.0, 100.0, 2000.0, 2000.0);
        let chat_box_rectangle = Rectangle::new(0.0, 0.0, SCREEN_SIZE[0] as f32, 300.0);
        let bullet_rectangle = Rectangle::new(0.0, 0.0, 15.0, 15.0);
        let player_rectangle = Rectangle::new(0.0, 0.0, PLAYER_WIDTH, PLAYER_WIDTH);
        unsafe {
            // Used during collision detection thread
            MAPS_RECTANGLE = maps_rectangle_stack;
        }

        Ok(GameState {  
            mouse_shape: GeometryBuilder::new() 
            .set_color(Color::rgb(0.0, 0.0, 1.0))
            .rectangle(ShapeStyle::Fill, Rectangle::new(0.0, 0.0, 18.0, 18.0))?
            .build_mesh(ctx)?,

            map_shape: GeometryBuilder::new()
            .set_color(Color::rgb(0.4, 0.6, 0.4))
            .rectangle(ShapeStyle::Fill, maps_rectangle_stack)?
            .build_mesh(ctx)?,        
            map_rect: maps_rectangle_stack,
            
            player_shape: Mesh::circle(ctx, ShapeStyle::Fill, Vec2::zero(), PLAYER_WIDTH)?,
            player_rect: player_rectangle,

            chat_box_shape: GeometryBuilder::new()
            .set_color(Color::rgba(0.3, 0.3, 0.3, 0.5))
            .rectangle(ShapeStyle::Fill, chat_box_rectangle)?
            .set_color(Color::rgb(0.0,0.0,0.0,))
            .rectangle(ShapeStyle::Stroke(5.0), Rectangle::new(0.0, 0.0, SCREEN_SIZE[0] as f32, 300.0))?
            .build_mesh(ctx)?,
            chat_box_rect: chat_box_rectangle,
            chat_box_line_shape: GeometryBuilder::new()
            .set_color(Color::rgba(0.0, 0.0, 0.0, 0.4))
            .rectangle(ShapeStyle::Fill, Rectangle::new(2.5, 0.0, chat_box_rectangle.width - 5.0, TEXT_SIZE+10.0))?
            .build_mesh(ctx)?,

            bullet_shape: GeometryBuilder::new()
            .set_color(Color::rgb(1.0, 0.0, 0.0))
            .rectangle(ShapeStyle::Fill, bullet_rectangle)?
            .build_mesh(ctx)?,
            
            bullet_rect: bullet_rectangle,
            

            current_active_bullets: Vec::new(),
            // Local players latest bullet
           // current_bullet_position: UNSET_BULLET,
            
            text: Text::new("-", Font::vector(ctx, "./res/style1.ttf", TEXT_SIZE)?),
            chat_mode: false,

            local_player_position: [0.0; 2],
            local_player_message: String::new(),

            scroll: [0.0; 2]
        })   
    }
}
impl State for GameState {
    fn draw(&mut self, ctx: &mut Context) -> tetra::Result {
        graphics::clear(ctx, Color::rgb(0.43, 0.24, 0.51));
        
        // Draw Arena
        self.map_shape.draw(ctx, Vec2::from([0.0 - self.scroll[0], 0.0 - self.scroll[1]]));

        // Draw Local Player
        self.player_shape.draw(ctx, Vec2::from([self.local_player_position[0] - self.scroll[0], self.local_player_position[1] - self.scroll[1]]));

        if self.chat_mode {
            // Draw Chat-box
            self.chat_box_shape.draw(ctx, Vec2::new(0.0, 0.0));
            self.chat_box_line_shape.draw(ctx, Vec2::new( self.chat_box_rect.x, self.chat_box_rect.height - TEXT_SIZE - 10.0));
            // Draw local message currently typing in
            self.text.set_content(self.local_player_message.as_str());
            self.text.draw(ctx, Vec2::new(13.0, self.chat_box_rect.height - TEXT_SIZE - 7.0));
            

            unsafe {
                let current_chat_log = CHAT_LOG.clone();
                // Draw Messages 
                for (index, msg) in current_chat_log.iter().rev().enumerate() {
                    self.text.set_content(msg);
                    let y = ((self.chat_box_rect.height - (TEXT_SIZE - 3.0)) - ((TEXT_SIZE + 3.0) * index as f32)) - ((TEXT_SIZE + 3.0)*2.0);
                    self.text.draw(ctx, Vec2::new(
                        13.0,
                        y - 3.0
                    ));
                }     
            }
        }
        unsafe {
            // Draw Other Players
            let other_players_details = PLAYERS_DETAILS.clone();
            for player in other_players_details.iter() {
                let mut xy:Vec2<f32> = Vec2::from(player.position);
                xy[0] = xy[0] - self.scroll[0];
                xy[1] = xy[1] - self.scroll[1];
                self.player_shape.draw(ctx, Vec2::from([xy.x, xy.y]));
              
                self.text.set_content(player.name.clone());
                // self.text.set_max_width(Some(50.0));
                self.text.draw(ctx, Vec2::from([xy[0] - ((player.name.len() as f32 * TEXT_SIZE)/2.0), xy[1] - 28.0]));
            }

            // Draw Other Players Bullets
            // let current_active_bullets_mutex = ACTIVE_BULLETS.lock().unwrap();
            // let current_active_bullets = current_active_bullets_mutex.clone();
            // std::mem::drop(current_active_bullets_mutex);
            // Guard drops once 'unsafe {}' scope here is finished

            for bullet in self.current_active_bullets.iter() {
                self.bullet_shape.draw(ctx, Vec2::from([bullet.rect.x - self.scroll[0], bullet.rect.y - self.scroll[1]]));
            }
            //println!("{:?}", current_active_bullets);
        }
        

        // Draw Mouse
        self.mouse_shape.draw(ctx, input::get_mouse_position(ctx));

        Ok(()) 
    }  
    fn update(&mut self, ctx: &mut Context) -> tetra::Result {

        // Bullet Collision Detection
        let current_local_rect:Rectangle = Rectangle::new(self.local_player_position[0], self.local_player_position[1], PLAYER_WIDTH, PLAYER_WIDTH);
        let mut bullets_to_remove:Vec<usize> = Vec::new();
        for (index, bullet) in self.current_active_bullets.clone().iter().enumerate() {
            let current_bullet_mut = &mut self.current_active_bullets[index];

            // Update Bullets Position
            current_bullet_mut.rect.x -= f32::cos(current_bullet_mut.direction) * current_bullet_mut.speed;
            current_bullet_mut.rect.y -= f32::sin(current_bullet_mut.direction) * current_bullet_mut.speed;
            
            current_bullet_mut.speed -= 0.06;

            if current_bullet_mut.speed <= 2.0 {
                bullets_to_remove.push(index);
            }
            else if current_local_rect.intersects(&bullet.rect) {
                // Bullet has hit player
                if self.current_active_bullets[index].players_name != "[me]" {
                    //   bullets_to_remove.push(index);
                    unsafe {
                        LOCAL_IS_HIT = true;
                    }


                    self.local_player_position = [(random::<f32>() * 10.0) * 6.0, (random::<f32>() * 10.0) * 6.0];
                }

            }
            else if !(bullet.rect.intersects(&self.map_rect)) {
                // Bullet is out of bounds of arena
                bullets_to_remove.push(index);
              //  if DEBUG { println!("Out of bounds: {:?}", bullet.rect); }
            }
        }
        // Throws error: thread 'main' panicked at 'removal index (is 1) should be < len (is 1)', src\main.rs:224:41
        let mut removal_offset: usize = 0;
        for i in bullets_to_remove {
            self.current_active_bullets.remove(i - removal_offset);
            removal_offset += 1;
        }
    
        self.scroll[0] += ((self.local_player_position[0] - self.scroll[0]) - (SCREEN_SIZE[0] as f32/2.0) as f32)/10.0;
        self.scroll[1] += ((self.local_player_position[1] - self.scroll[1]) - (SCREEN_SIZE[1] as f32/2.0))/10.0;
        
        let mut speed:f32 = 0.0;
        if input::is_key_down(ctx, Key::LeftShift) {
            speed = 2.0;
        }
        if input::is_key_down(ctx, Key::W) {
            self.local_player_position[1] -= MOVEMENT_SPEED + speed;
        }
        else if input::is_key_down(ctx, Key::S) {
            self.local_player_position[1] += MOVEMENT_SPEED + speed;
        }
        if input::is_key_down(ctx, Key::A) {
            self.local_player_position[0] -= MOVEMENT_SPEED + speed;
        }
        else if input::is_key_down(ctx, Key::D) {
            self.local_player_position[0] += MOVEMENT_SPEED + speed;
        }

        if input::is_mouse_button_released(ctx, input::MouseButton::Left) {
            // Local shoots new bullet
            let mouse_pos = input::get_mouse_position(ctx);
            let path = f32::atan2(
                ((self.local_player_position[1] - (self.player_rect.width/2.0)) - mouse_pos.y) - self.scroll[1],
               ((self.local_player_position[0] - (self.player_rect.width/2.0)) - mouse_pos.x) - self.scroll[0]
            );
           

            self.current_active_bullets.push(
                PlayersBullet { 
                    players_bullet_id: "[me]".to_string(),
                    players_name: ("[me]".to_string()), 
                    rect: (Rectangle::new(self.local_player_position[0], self.local_player_position[1], BULLET_WIDTH, BULLET_WIDTH)), 
                    direction: (path), 
                    speed: (7.0) 
                }
            );        
            unsafe {
                LOCAL_DETAILS.recent_bullet_position_and_direction = [self.local_player_position[0], self.local_player_position[1], path];  
                LOCAL_DETAILS.new_bullet_id += 1;
            }
        }

        if input::is_key_down(ctx, Key::Escape) {
            unsafe {
                //LOCAL_DESIRES_CONNECTED = false;
            }
        }
        if input::is_key_released(ctx, Key::T) {
            self.chat_mode = true;
        }

        // Arena Collision Detection
        if self.local_player_position[0] <= self.map_rect.x {
            self.local_player_position[0] = self.map_rect.x
        }
        if self.local_player_position[1] <= self.map_rect.y {
            self.local_player_position[1] = self.map_rect.y
        }
        if self.local_player_position[0] >= (self.map_rect.x + self.map_rect.width) {
            self.local_player_position[0] = self.map_rect.x + self.map_rect.width
        }
        if self.local_player_position[1] >= (self.map_rect.y + self.map_rect.height) {
            self.local_player_position[1] = self.map_rect.y + self.map_rect.height
        }

        // Typing in chat
        if self.chat_mode {
            // Player has chat open
            let wrapped_value = input::get_text_input(ctx);
            match wrapped_value {
               Some(val) => {self.local_player_message.push_str(val);}
               None => {}
            }
            if input::is_key_released(ctx, Key::Backspace) {
                self.local_player_message.pop();
            }
            if input::is_key_down(ctx, Key::Escape) {
                self.chat_mode = false;
            }
            if input::is_key_released(ctx, Key::Enter) {
                unsafe {
                    CHAT_LOG.push(format!("[{}]: {}", LOCAL_DETAILS.name, self.local_player_message.clone()));
                    LOCAL_DETAILS.message = format!("'{}'", self.local_player_message.clone());
                }
                self.local_player_message = "".to_string();
            }
        }
        
        // Setting global var to be used in 'server_handle' thread
        unsafe {
            LOCAL_DETAILS.position = self.local_player_position.clone();
            
            let mut new_bullets_mutex = NEW_BULLETS_BUF.lock().unwrap();
            let new_bullets = new_bullets_mutex.clone();
            new_bullets_mutex.clear();
            std::mem::drop(new_bullets_mutex);
            
            
            for new_bullet in new_bullets {
                self.current_active_bullets.push(new_bullet);
            }
        }
        Ok(())
    }
}
/// Returns built tetra application
fn setup_window() -> tetra::Result {
    ContextBuilder::new("Online Squares!", SCREEN_SIZE[0], SCREEN_SIZE[1])
        .build()?
        .run(GameState::new)    
}

fn main() {
    unsafe {
        // Username cannot contain server key's e.g. ':'
        LOCAL_DETAILS.name = "user".to_string();

        PLAYERS_MESSAGES.resize(MAX_PLAYERS, PlayersMessage {id:(usize::MAX), msg:("".to_string())})
    }
    thread::spawn(server_handle);
   //  thread::spawn(bullet_collision_detection);
    let _ = setup_window();    
}

const X:usize = 0;
const Y:usize = 1;

/* SERVER COMMUNICATIONS */
/// TCP Stream for current client & server communications of data \ 
/// Handled by its own thread
fn server_handle() {
    let stream = TcpStream::connect("");
    
    match stream {
        Ok(mut stream) => {
            loop {
                /* RECEIVING DATA FROM SERVER */               
                let mut raw_receive_data:[u8; 550] = [0u8; 550];
                let _ = stream.read(&mut raw_receive_data);
                let data = std::str::from_utf8(&raw_receive_data);
                match data {
                    Ok(msg) => {
                        let actual_data;
                        match msg.find("~") {
                            Some(cut_off) => { 
                                actual_data = msg[0..cut_off].to_string(); 
                                unsafe { 
                                    PLAYERS_DETAILS = get_players_from_string(actual_data); 
                                }
                            }
                            None => {}
                        }  
                    }
                    Err(e) => {println!("[ERROR]: {}", e);}
                }

                /* SENDING LOCAL DETAILS TO SERVER - (X,Y Position) */
                let send_val:String;
                unsafe {
                    if !LOCAL_DESIRES_CONNECTED {
                        // Leave server
                        let _ = stream.write("(DISCONNECT)".as_bytes());
                        break;
                    }
                    let local_details_copy = LOCAL_DETAILS.clone();

                    send_val = get_string_from_local_details(local_details_copy);
                    // Clear local message content
                    LOCAL_DETAILS.message = "".to_string();
                    //LOCAL_DETAILS.recent_bullet_position_and_direction = UNSET_BULLET;
                }
                let _ = stream.write(send_val.as_bytes());
                
            }
        }
        Err(e) => { 
            panic!("[ERROR]: {}", e)
        }
    }
}

/* SUB FUNCTIONS USED FOR 'server_handle' */
/// Prepares string which will be sent over to the server \
/// Returns prepared String
fn get_string_from_local_details(local_player_details: LocalPlayerDetails) -> String {
    format!("{}:{},{}:{}]{},{},{}:{}~ ",
    local_player_details.name,
    local_player_details.position[0], local_player_details.position[1], 
    local_player_details.new_bullet_id, local_player_details.recent_bullet_position_and_direction[0], local_player_details.recent_bullet_position_and_direction[1], local_player_details.recent_bullet_position_and_direction[2],
    
    local_player_details.message
    )  
}
/// Gathers each players details from string received & processes it to Vec<[f32; 2]> \
/// Returns processed Vector
fn get_players_from_string(data: String) -> Vec<GlobalPlayerDetails> {
    let mut ret: Vec<GlobalPlayerDetails> = Vec::new();
    ret.resize(20, GlobalPlayerDetails { id: (usize::MAX), name: (String::from("")), position: ([f32::MAX; 2]), recent_bullet_id: (String::from("")) });
    let mut new_bullets_gathered: Vec<PlayersBullet> = Vec::new();
  //  let previous_player_details: Vec<GlobalPlayerDetails>;
    
    let mut ids_found: String = String::new();
    let previous_ids_found:String; 
    let previous_player_details: Vec<GlobalPlayerDetails>;
    unsafe {
        previous_ids_found = PREVIOUS_PLAYER_FOUND.clone();
        previous_player_details = PLAYERS_DETAILS.clone();
    }

    let player_values = data.split(";");
    for val in player_values.into_iter() {
        if !val.contains("|") && !val.is_empty() {
            // Player slot is active
            let mut player_details = GlobalPlayerDetails {id: (usize::MAX), name: ("[Unknown User]".to_string()), position: ([-1000.0, -1000.0]), recent_bullet_id: (String::from("0"))};
            let values = val.split(":");
            
            let mut data_is_corrupt = false;
            let mut player_id_found:bool = false;
            let mut players_message = "".to_string();
            for (value_index, j) in values.into_iter().enumerate() {
                match value_index {
                    // Handle each value here: 
                    0 => { 
                        //Player ID
                        match j.parse::<usize>() {
                            Ok(r) => { player_details.id = r; player_id_found = true; ids_found.push_str(format!("[{}]", r).as_str()) }
                            Err(e) => { println!("{}", e); } 
                        }                    
                    } 
                    1 => {player_details.name = j.to_string();}
                    2 => {player_details.position = extract_player_position(j.trim().to_string());}
                    3 => { 
                        
                        if previous_ids_found.contains(&format!("{}", player_details.id).to_string()) {
                            match extract_player_bullet_info(j.trim().to_string()) {
                                Some(bullet_data) => {
                                    
                                    // if DEBUG {println!("Bullet info: {:?}", r); }
                                    
                                    let players_previous_details = previous_player_details[player_details.id].clone();
                                    player_details.recent_bullet_id = players_previous_details.recent_bullet_id.clone();

                                    if !(bullet_data.players_bullet_id.eq(&player_details.recent_bullet_id)) {
                                        // println!("new bullet! - new: {} old {}", bullet_data.players_bullet_id, player_details.recent_bullet_id);
                                        
                                        player_details.recent_bullet_id = bullet_data.players_bullet_id.clone();
                                        new_bullets_gathered.push(bullet_data);
                                    } 
                                }
                                None => {}
                            }
                        }
                    }
                    4 => { 
                        players_message = j.to_string();
                    }
                    _ => {println!("Corrupt data received? - {}", data); data_is_corrupt = true;}
                }   
            }
            if !data_is_corrupt {
                if !players_message.is_empty() && player_id_found { 
                    unsafe {
                        // stop spamming messages
                        // If players recent message is same as this message received
                        if !(PLAYERS_MESSAGES[player_details.id].msg == players_message){
                         //   if DEBUG { println!("[{}]{}: {}", player_details.id, player_details.name, players_message); }
                            // Push to chat log
                            if CHAT_LOG.len() >= 20 {
                                CHAT_LOG.remove(0);
                            }
                            CHAT_LOG.push(format!("[{}]: {}", player_details.name, players_message.clone()));
                            // To keep track of this players message history for spam
                            PLAYERS_MESSAGES[player_details.id].msg = players_message;
                        }
                        // Else Ignore
                    } 
                } 
                else if !players_message.is_empty() {
                    println!("Could not find player id? - {}", val)
                }
            }
            // Should be the id just gathered from server - (This should be a reserved spot for this player)
            let id = player_details.id.clone();
            ret[id] = player_details;
        }
    }
    // Push new collected bullets to buffer
    unsafe {
        let mut new_bullets_buf_mutex = NEW_BULLETS_BUF.lock().unwrap();
        for new_bullet in new_bullets_gathered {
            new_bullets_buf_mutex.push(new_bullet);
        }
        PREVIOUS_PLAYER_FOUND = ids_found
        // Guard drops once 'unsafe {}' scope here is finished
    }

    ret
}
/// EXTRACTS X,Y VALUES FROM STRING \
/// Returns: [x, y] as [f32; 2]
fn extract_player_position(data:String) -> [f32; 2] {
    let mut ret:[f32; 2] = [0.0; 2];
    let mut xy_str:[String; 2] = ["".to_string(), "".to_string()];
    let mut xy_index = 0;
    for char in data.chars() {
        if char == ',' {
            // is a seperator for x,y values
            xy_index = 1;
        }
        else if char.is_numeric() || char == '.' || char == '-' {
            xy_str[xy_index].push(char);
        }
    }
    for (index, val) in xy_str.iter().enumerate() {
        let parse_result = val.parse::<f32>();
        match parse_result {
            Ok(parse_val) => {ret[index] = parse_val;}
            Err(_e) => {}//println!("{}", e)}
        }
    }
    ret  
}
/// EXTRACTS NEW BULLETS X,Y,DIRECTION VALUES FROM STRING \
/// Returns: PlayersBullet { ID, players_name, xy, direction, speed }
fn extract_player_bullet_info(data:String) -> Option<PlayersBullet> {
    let mut ret:PlayersBullet = PlayersBullet { players_bullet_id: ("".to_string()), players_name: ("".to_string()), rect: (Rectangle::new(0.0, 0.0, BULLET_WIDTH, BULLET_WIDTH)), direction: (0.0), speed: (10.0) };
    // x, y, direction
    let mut xyd_values:[f32; 3] = [f32::MAX; 3]; 
    // ADD ID, COLLECT IT HERE AND REMOVE BEFORE BELOW
    let id_and_data = data.split("]");
    let mut actual_data:String = "".to_string();
    if id_and_data.clone().count() == 2 {
        for (index, i) in id_and_data.into_iter().enumerate() {
            match index {
                0 => { ret.players_bullet_id = i.to_string(); }
                1 => { actual_data = i.clone().to_string(); }
                _ => { println!("Found extra values? - {}", data); }
            }
        }
    }
    else {
        println!("Couldnt find id seperation for Players Bullet? - {}", data);
    }

    let x_y_direction_values = actual_data.split(',');
    if x_y_direction_values.clone().count() == 3 {
        for (index, val) in x_y_direction_values.into_iter().enumerate() {
            let parse_result = val.parse::<f32>();
            match parse_result {
                Ok(r) => { 
                    if r == f32::MAX {
                        // Not a new bullet value - A normal bullet should never get to this value
                        return None
                    }
                    else {
                        xyd_values[index] = r;
                    }
                }
                Err(e) => {println!("{}", e)}
            }
        }
    }
    else {
        println!("Bullet data came through corrupt? - {}", data);
        return None
    }

    ret.rect.x = xyd_values[0]; 
    ret.rect.y = xyd_values[1];
    ret.direction = xyd_values[2];
    ret.speed = 10.0;

    Some(ret)
}