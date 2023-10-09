# About
Youtui - a simple TUI YouTube Music player written in Rust.
Ytmapi-rs - an asynchronous API for youtube music - using Google's internal API.

This project is not supported by Google.
## Coding constraints
App has been designed for me to learn Rust, and therefore I have implemented the following constraints to learn some features.
1. Avoid shared mutable state: 
The app will avoid shared mutable state primitives such as Mutex and RefCell and instead communicate via messaging.
1. Concurrency over parralelism: 
Where possible, the app will use use an asynchronous mode of operation (such as futures::join! and tokio::select) over parallel equivalents such as tokio::spawn and thread::spawn.
1. Avoid cloning: Where possible, the app will avoid cloning as a method to beat the borrow checker. Instead, we will try to safely borrow.
1. Encode state into the type system: Where possible use the type system to represent actions that are not possible in the current state. This will improve developer ergonomics.
## Design constraints
I am aiming to follow the following design principles
1. Smart defaults: 
Where defaults are implemented, they should be set in a way that is best for the user. 
1. Discoverability
Where a command is not default, or does not follow the principle of least surprise, the command should be visible to the user. For example, F2 to search. Commands that require multiple keypresses should display context menus for the subsequent presses (e.g, see Kakoune or Helix).
## Roadmap
### Application
- [ ] Offline cache
- [ ] Implement improved download speed
- [ ] Real time streaming
- [ ] Theming
### API
- [ ] Implement all endpoints
- [ ] OAuth authentication

| Endpoint | Implemented |
| --- | --- |
| Get | [ ] |
| Search | [ ] |
| | |
