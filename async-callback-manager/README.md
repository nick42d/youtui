# About
async-callback-manager is a crate design to help you manage an asynchronous callback pattern when developing Rust UI.

This is initially designed as a dependency of the `youtui` music player, however more documentation will follow.
# Basic usage

The AsyncCallbackManager should live with your event loop, and can produce a stream of events corresponding to a component callback or an asynchronous task.

From the AsyncCallbackManager you can create AsyncCallbackSenders that allow you to log asynchronous callbacks and receive their replies as a list of state mutations to be applied.
