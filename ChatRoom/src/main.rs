use std::{net::{TcpListener, TcpStream}, thread, io::{Write, Read, Split}};
use std::time::Duration;

// Possibly used for advancing networking in future
use default_net; 

const DEBUG:bool = true;

#[derive(Clone, Debug)]
struct PlayerDetails {
    id: usize,
    name: String,
    pos: String,
    bullet_info: String,
    msg: String
}

static mut PLAYERS_DETAILS:Vec<PlayerDetails> = Vec::new();
const MAX_USERS:usize = 5;
static mut ACTIVE_PLAYERS_COUNT:usize = 0;
const UNSET_BULLET:String = String::new();
const BYTES_PER_CLIENT_MSG:usize = 455;

// KEY: 
// Example string - "12.1,51.2 : 'message' : value ; "12.1,51.2 : 'message' : value" 
// ; - End of a players details
// : - Seperation of a PlayersDetails value
// , - Seperation of multiple sub-values within a PlayersDetails value
// ' - Ecapsulation of a string value 
// SERVER SIDE VALUES
// | - Player spot EMPTY
// ~ - End of string in buffer received from client  


fn handle_connection(mut client: TcpStream) {
    let mut players_id:usize = usize::MAX;
    unsafe {
        for (index, val) in PLAYERS_DETAILS.iter().enumerate(){
            if val.name.contains("|") {
                // Spot Empty
                // Bring index back down to ZERO range
                players_id = index;
                PLAYERS_DETAILS[index].id = index;
                
                break;
            }
        }
        if players_id == usize::MAX {
            panic!("[SERVER ERROR]: Cannot find empty spot for player. One of the previous player 
                strings may have become corrupt? - {:?}", PLAYERS_DETAILS); 
        }
        ACTIVE_PLAYERS_COUNT += 1;
    }
    println!("[SERVER]: New connection: {}", players_id);
    
    loop {        
        // Send ALL Clients details
        let send_val:String;
        unsafe {
            let mut other_players_details:Vec<PlayerDetails> = PLAYERS_DETAILS.clone();
            other_players_details.remove(players_id);
            send_val = stringvec_to_string(other_players_details);   
        }
       
        let write_result = client.write(send_val.as_bytes());
        match write_result {
            Ok(_r) => {}
            Err(e) => {
                // ADD TIME-OUT Instead of cutting connection straight away
                // Connection is no longer existent - (Local may have abruptly lost connection or forcibly left)
                if DEBUG {println!("[SERVER ERROR]: {}", e);}
                handle_disconnect(players_id);
                break;
            }
        }
        
        // Read clients details
        // This is a buffer for the bytes obtained/read throughout this stream
        let mut receive_data:[u8; BYTES_PER_CLIENT_MSG] = [0u8; BYTES_PER_CLIENT_MSG];
        let _ = client.read(&mut receive_data);
        let received_data_unpacked = std::str::from_utf8(&receive_data);
        
        match received_data_unpacked {
            Ok(msg) => {
                if msg.contains("(DISCONNECT)") {
                    handle_disconnect(players_id);
                    break;
                }
                
                let msg_cutoff = msg.find("~");
                match msg_cutoff {
                    Some(cutoff) => { 
                        unsafe {
                            let actual_msg = msg[0..cutoff].to_string(); 
                       
                            // Split values from message into players values
                            let players_details = actual_msg.split(":");
                            for (index, val) in players_details.into_iter().enumerate() {
                                match index {
                                    0 => { PLAYERS_DETAILS[players_id].name = val.to_string(); }
                                    1 => { PLAYERS_DETAILS[players_id].pos = val.to_string(); }
                                    2 => { PLAYERS_DETAILS[players_id].bullet_info = val.to_string(); }
                                    3 => { 
                                        if val.matches("'").count() >= 2 {
                                            if val != "''" {
                                                println!("[{}] {}", PLAYERS_DETAILS[players_id].name, val);
                                                PLAYERS_DETAILS[players_id].msg = val.to_string(); 
                                            }  
                                        }
                                    }
                                    _ => { println!("To many player details values received? - {}", actual_msg); }
                                }
                            }       
                        }
                    }
                    None => { println!("Couldn't manipulate data? - {}", msg); }
                }
            }
            Err(e) => {
                println!("ERROR: {}", e); break;
            }
        }
    }
}

fn main() {
    unsafe {
        PLAYERS_DETAILS.resize(MAX_USERS, PlayerDetails { id: (usize::MAX), name: ("|".to_string()), pos: ("0.0,0.0".to_string()), bullet_info: (UNSET_BULLET), msg: ("".to_string()) });
    }
    let host = "";
    let listener_result = TcpListener::bind(host);


    match listener_result {
        Ok(listener) => {
            println!("[SERVER]: Now Listening for connections on {}", host);
            for incoming_stream in listener.incoming() {
                match incoming_stream {
                    Ok(s) => { 
                        let current_active_players_count: usize;
                        let current_players_details: Vec<PlayerDetails>;
                        unsafe {
                            current_active_players_count = ACTIVE_PLAYERS_COUNT.clone();
                            current_players_details = PLAYERS_DETAILS.clone();
                        }
                        if current_active_players_count >= current_players_details.len() {
                            println!("Client attempted to join but lobby has reached maximum limit");
                        } else {
                            // Add new connection as there's available space
                            thread::spawn(|| {
                                handle_connection(s);
                            });
                        } 
                    },
                    Err(e) => { println!("[SERVER]: ERROR DURING CONNECTION - {}", e) }
                }
            }
        }
        Err(e) => {
            println!("[SERVER]: {}", e);
        }
    }
}
/// Converts Vector of player details to one string for transmission
fn stringvec_to_string(arr:Vec<PlayerDetails>) -> String {
    let mut ret:String = "".to_string();
    for player in arr.iter() {
        ret.push_str(format!("{}:{}:{}:{}:{};", 
        player.id,
        player.name, 
        player.pos, 
        player.bullet_info,
        player.msg
        ).as_str());
    }
    // Add '~' to determine end of all players data
    ret.push('~');
    ret
}
/// Executes on disconnection of a client
fn handle_disconnect(player_id:usize) {
    println!("[SERVER]: Connection lost: {}", player_id);
    unsafe {
        ACTIVE_PLAYERS_COUNT -= 1;
        PLAYERS_DETAILS[player_id].name = "|".to_string();
    };
}

// fn f32vec_to_string(arr: Vec<[f32; 2]>) -> String
// {
//     let mut ret:String = "#".to_string();
    
//     for i in 0..arr.len() {
//         let values = arr[i];
//         ret.push_str(format!("{},{}; ", values[0], values[1]).as_str());
//     };

//     ret
// }