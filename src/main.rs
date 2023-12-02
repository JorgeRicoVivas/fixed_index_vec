use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::ops::Deref;
use std::thread::sleep;
use std::time::Duration;

use crate::internal::FixedIndexVec;

struct Endmark {
    string: &'static str,
    escape: &'static str,
}

const ENDMARK: Endmark = Endmark {
    string: "EOF",
    escape: "\\EOF",
};

fn main() {
    /*
    let mut vec_assing = VecAssign::new(["Position zero", "Position one", "Position two", "Position three"]);
    println!("{:?}", vec_assing.remove(2));
    println!("{:?}", vec_assing.remove(1));
    let pos = vec_assing.reserve_pos();
    vec_assing.push("New pos 2");
    vec_assing.push_reserved(pos, "New pos 1");

    println!("{:?}", vec_assing);
    */

    /*
    let input = " asiuh EOF asd ";
    println!("{:?}", find_message_end_bound_utf16(input, input.len(), false));
    */


    let listener = TcpListener::bind("127.0.0.1:5050").unwrap();
    println!("{}", listener.set_nonblocking(true).is_ok());
    let mut server = SimpleServer::<u16>::new(listener);
    server.on_get_message(|server, client_id, message| {
        println!("Got message from {}, contents are:", client_id);
        println!("{message}");
        println!("{:?}", server.send_message_to(client_id, &*format!("You sent: {message}")));
    });
    loop {
        println!("---New cycle---");
        println!("-Accepting clients-");
        server.accept();
        println!("-Reading clients-");
        server.read_clients();
        println!("---End of cycle---");
        sleep(Duration::from_secs(1));
    }
}

fn substring_utf16(original_string: &str, start_index: usize, end_index: usize) -> String {
    String::from_utf16(&*original_string.chars().take(end_index).skip(start_index).map(|char| char as u16).collect::<Vec<u16>>()).unwrap()
}

fn find_message_end_bound_utf16(input: &str, start_checking_from: usize, check_from_left_to_right: bool, up_to: usize) -> Option<usize> {
    if input.len() < ENDMARK.string.len() { return None; }
    let endmark = ENDMARK.string;
    let escape_endmark = ENDMARK.escape;
    let desired_endmark_queue = endmark.chars().collect::<Vec<char>>();
    let desired_escape_endmark_queue = escape_endmark.chars().collect::<Vec<char>>();
    let start_checking_from = start_checking_from.max(endmark.len()).min(input.len());
    let up_to = up_to.max(0).min(input.len());

    let mut buffered_endmark = VecDeque::with_capacity(endmark.len());
    let mut buffered_escape_endmark = VecDeque::with_capacity(escape_endmark.len());

    if check_from_left_to_right {
        let mut endmark_iter = input.chars();
        let mut escape_iter = input.chars();

        let starting_to_read_index = start_checking_from.checked_sub(endmark.len().max(escape_endmark.len())).unwrap_or(0);
        let mut character_index = 0;
        while character_index < input.len() {
            character_index += 1;
            let endmark_char = endmark_iter.next();
            let escape_char = escape_iter.next();
            if character_index < starting_to_read_index {
                continue;
            }
            insert_on_queue(&mut buffered_endmark, endmark_char.unwrap(), check_from_left_to_right);
            insert_on_queue(&mut buffered_escape_endmark, escape_char.unwrap(), check_from_left_to_right);
            if character_index < start_checking_from {
                continue;
            }
            let is_endmark = buffered_endmark.eq(&desired_endmark_queue);
            let is_escape_endmark = buffered_escape_endmark.eq(&desired_escape_endmark_queue);
            println!("From left: {buffered_endmark:?}, {buffered_escape_endmark:?}, {is_endmark}, {is_escape_endmark}");
            let real_index = character_index - endmark.len();
            if is_endmark && !is_escape_endmark {
                return Some(real_index);
            }
            if real_index > up_to {
                return None;
            }
        }
    } else {
        let mut endmark_iter = input.chars().rev();
        let mut escape_iter = input.chars().rev();

        let starting_to_read_index = start_checking_from;
        let mut character_index = input.len();
        loop {
            let endmark_char = endmark_iter.next();
            let mut escape_char = escape_iter.next();
            let mut ignore_escape = escape_char.is_none();
            if character_index > starting_to_read_index {
                if character_index == 0 {
                    return None;
                }
                character_index -= 1;
                continue;
            }
            if character_index == starting_to_read_index {
                let n = escape_endmark.len() - endmark.len();
                if n > 0 {
                    if escape_char.is_none() { break; }
                    insert_on_queue(&mut buffered_escape_endmark, escape_char.unwrap(), check_from_left_to_right);
                    for i in 1..n {
                        let next_char = escape_iter.next();
                        if next_char.is_none() { break; }
                        insert_on_queue(&mut buffered_escape_endmark, next_char.unwrap(), check_from_left_to_right);
                    }
                    escape_char = escape_iter.next();
                    ignore_escape = escape_char.is_none();
                }
            }
            insert_on_queue(&mut buffered_endmark, endmark_char.unwrap(), check_from_left_to_right);
            if escape_char.is_some() {
                insert_on_queue(&mut buffered_escape_endmark, escape_char.unwrap(), check_from_left_to_right);
            }
            let is_endmark = buffered_endmark.eq(&desired_endmark_queue);
            let is_escape_endmark = !ignore_escape && buffered_escape_endmark.eq(&desired_escape_endmark_queue);
            if character_index != 0 {
                character_index -= 1;
            }
            println!("From right: {buffered_endmark:?}, {buffered_escape_endmark:?}, {is_endmark}, {is_escape_endmark}");
            if is_endmark && !is_escape_endmark {
                return Some(character_index);
            }
            if character_index <= up_to {
                return None;
            }
        }
    }

    None
}

fn insert_on_queue<T>(queue: &mut VecDeque<T>, value: T, front_to_back_order: bool) {
    let remove: fn(&mut VecDeque<T>) -> Option<T> = if front_to_back_order { VecDeque::pop_front } else { VecDeque::pop_back };
    let push: fn(&mut VecDeque<T>, T) = if front_to_back_order { VecDeque::push_back } else { VecDeque::push_front };
    if queue.len() >= queue.capacity() {
        remove(queue);
    }
    push(queue, value);
}


fn find_and_process_messages(input: &mut String, mut start_checking_from: usize, mut action: impl FnMut(&str, &mut bool)) {
    let mut keep_checking = true;
    while keep_checking {
        let end_of_message_index = find_message_end_bound_utf16(input, start_checking_from, true, input.len());
        if end_of_message_index.is_none() { return; }
        let end_of_message_index = end_of_message_index.unwrap();
        let message = substring_utf16(input, 0, end_of_message_index).replace(ENDMARK.escape, ENDMARK.string);
        action(&message, &mut keep_checking);
        *input = substring_utf16(input, end_of_message_index + ENDMARK.string.len(), input.len());
        start_checking_from = 0;
    }
}

pub mod internal;


struct SimpleServer<ClientData> {
    server_socket: TcpListener,
    clients: FixedIndexVec<Client<ClientData>>,
    on_accept: fn(&Self, usize) -> Option<ClientData>,
    on_get_message: fn(&mut Self, usize, &str),
    endmark: Endmark,
}

impl<ClientData> SimpleServer<ClientData> {
    pub fn new(listener: TcpListener) -> SimpleServer<ClientData> {
        Self {
            server_socket: listener,
            clients: FixedIndexVec::new(),
            on_accept: |_, _| { None },
            on_get_message: |_, _, _| {},
            endmark: ENDMARK,
        }
    }

    pub fn on_accept(&mut self, on_accept: fn(&Self, usize) -> Option<ClientData>) { self.on_accept = on_accept; }

    pub fn on_get_message(&mut self, on_get_message: fn(&mut Self, usize, &str)) {
        self.on_get_message = on_get_message;
    }

    pub fn accept(&mut self) -> Option<()> {
        let (client_stream, client_socket) = self.server_socket.accept().ok()?;
        self.accept_client(client_stream, client_socket);
        Some(())
    }

    pub fn accept_incoming(&mut self) {
        self.server_socket.incoming()
            .filter(Result::is_ok)
            .map(Result::unwrap)
            .collect::<Vec<_>>().into_iter()
            .for_each(|client_stream| {
                let client_address = client_stream.peer_addr();
                if client_address.is_err() { return; }
                self.accept_client(client_stream, client_address.unwrap());
            });
    }

    fn accept_client(&mut self, stream: TcpStream, socket: SocketAddr) {
        let id = self.clients.reserve_pos();
        stream.set_nonblocking(true);
        let mut client = Client { id, stream, socket, message_buffer: String::new(), data: None };
        client.data = (self.on_accept)(self, id);
        if client.data.is_some(){
            self.clients.push_reserved(client.id, client);
        }else{
            self.clients.remove_reserved_pos(client.id);
        }
    }

    pub fn read_clients(&mut self) {
        let clients_len = self.clients.len();
        let mut client_index = 0;
        while client_index < clients_len {
            if !self.clients.contains_index(client_index) {
                client_index += 1;
                continue;
            }
            let client = self.clients.get_mut(client_index).unwrap();
            let mut stream_read = [0; 1024];
            let result = client.stream.read(&mut stream_read);
            match result {
                Ok(read_bytes) => {
                    let client_suddenly_disconnected = read_bytes == 0;
                    if client_suddenly_disconnected {
                        //The client has discconected without notifying it's connection's end,
                        //this happens when its program was closed forcedly
                        self.remove_client(client_index);
                        println!("Disconnected");
                        client_index += 1;
                        continue;
                    }
                    match String::from_utf16(&stream_read.map(|character| character as u16)) {
                        Ok(received_string) => {
                            self.read_clients_input(client_index, &received_string[0..read_bytes]);
                        }
                        Err(error) => {
                            client_index += 1;
                        }
                    }
                }
                Err(error) => {
                    println!("Error is {:?}", error.kind());
                    client_index += 1;
                }
            }
        }
    }

    fn read_clients_input(&mut self, client_index: usize, real_received_string: &str) {
        let client = self.clients.get_mut(client_index).unwrap();
        let message = real_received_string;
        let mut input = &mut client.message_buffer;
        let previous_input_len = input.len();
        input.extend(message.chars());
        let end_bound = find_message_end_bound_utf16(&input, input.len(), false,
                                                     previous_input_len.checked_sub(self.endmark.string.len() + 1).unwrap_or(0));
        if end_bound.is_none() { return; }
        let end_bound = end_bound.unwrap();
        let mut messages = substring_utf16(input, 0, end_bound + self.endmark.string.len());

        let buffer = substring_utf16(input, end_bound + self.endmark.string.len(), input.len());
        client.message_buffer = buffer;

        find_and_process_messages(&mut messages, 0, |message, keep_checking| {
            (self.on_get_message)(self, client_index, message);
            if !self.clients.contains_index(client_index) {
                *keep_checking = false;
            }
        });
    }

    pub fn remove_client(&mut self, client_id: usize) -> Option<Client<ClientData>> {
        self.clients.remove(client_id)
    }

    pub fn send_message_to(&mut self, client: usize, message: &str) -> Result<std::io::Result<usize>, ()> {
        let client = self.clients.get_mut(client);
        if client.is_none() { return Err(()); }
        let mut message = message.replace(self.endmark.string, self.endmark.escape);
        message.extend(self.endmark.string.chars());
        Ok(client.unwrap().stream.write(message.as_bytes()))
    }
}

struct Client<ClientData> {
    id: usize,
    stream: TcpStream,
    socket: SocketAddr,
    message_buffer: String,
    data: Option<ClientData>,
}

impl<ClientData> Client<ClientData> {

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn address(&self) -> &SocketAddr {
        &self.socket
    }

    pub fn data(&self) -> &ClientData {
        self.data.as_ref().expect("Tried to get Client's data while Client hasn't been initialized")
    }

    pub fn data_mut(&mut self) -> &mut ClientData {
        self.data.as_mut().expect("Tried to get Client's data while Client hasn't been initialized")
    }

}