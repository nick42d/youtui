pub trait JsonCrawler<'a, 'b>: Sized {
    fn get_source_ref(&self) -> &'a serde_json::Value;
    fn get_pointer_mut_ref(&mut self) -> Option<&'b mut serde_json::Value>;
    fn set_pointer(&mut self, json_pointer: &'b mut serde_json::Value);
    fn get_pointer_mut_ref_from_path(&mut self, path: &str) -> Option<&'b mut serde_json::Value>;
    fn get_path(&self) -> String;
    fn new(
        source_ref: &'a serde_json::Value,
        pointer_ref: &'b mut serde_json::Value,
        path: String,
    ) -> Self;
    fn take_json_pointer(&mut self, path: &str) -> Result<serde_json::Value> {
        let full_path = format!("{}{}", self.get_path(), path);
        let pointer = self.get_pointer_mut_ref().expect("test");
        let taken = pointer
            .pointer_mut(path)
            .map(|v| v.take())
            .ok_or_else(|| Error::navigation(full_path, self.get_source_ref()));
        self.set_pointer(pointer);
        taken
    }
    fn temp_navigate_pointer(&mut self, path: &str) -> Result<Self> {
        let full_path = format!("{}{}", self.get_path(), path);
        let json_pointer = self
            .get_pointer_mut_ref()
            .expect("test")
            .pointer_mut(path)
            .ok_or_else(|| Error::navigation(&full_path, &self.get_source_ref()))?;
        Ok(Self::new(self.get_source_ref(), json_pointer, full_path))
    }
}

impl<'a, 'b> JsonCrawler<'a, 'b> for BasicNav<'a, 'b> {
    fn get_pointer_mut_ref_from_path(&mut self, path: &str) -> Option<&'b mut serde_json::Value> {
        let mut pointer = self.json_pointer.take();
        let returner = pointer.as_mut().and_then(|p| p.pointer_mut(path));
        self.json_pointer = pointer;
        returner
    }
    fn set_pointer(&mut self, json_pointer: &'b mut serde_json::Value) {
        // The replaced value will always be None, we do not need it returned back to us.
        let _ = self.json_pointer.replace(json_pointer);
    }
    fn get_source_ref(&self) -> &'a serde_json::Value {
        self.json_debug
    }

    // Consider if this should always take a path.
    fn get_pointer_mut_ref(&mut self) -> Option<&'b mut serde_json::Value> {
        self.json_pointer.take()
    }
    fn get_path(&self) -> String {
        self.path.clone()
    }
    fn new(
        source_ref: &'a serde_json::Value,
        pointer_ref: &'b mut serde_json::Value,
        path: String,
    ) -> Self {
        BasicNav {
            json_debug: source_ref,
            json_pointer: Some(pointer_ref),
            path,
        }
    }
}
// could be a trait
struct BasicNav<'a, 'b> {
    json_debug: &'a serde_json::Value,
    json_pointer: Option<&'b mut serde_json::Value>,
    path: String,
}
