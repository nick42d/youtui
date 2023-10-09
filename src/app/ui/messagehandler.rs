// An application component that wants to receive responses to its messages.
// XXX: Not sure if this needs to exist. We don't care so much about the routing, more about being about to kill messages.
trait MessageHandler {
    // Message may implement something
    type Message;
    fn handle_message(&mut self, msg: Self::Message);
    // Kill any inbound messages for the handler
    fn kill_all(&mut self);
    // The component should have a unique identifier so that it can kill tasks.
    fn guid(&self) -> usize;
    fn is_guid(&self, guid: usize) -> bool {
        guid == self.guid()
    }
}
