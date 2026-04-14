trait Greeter {
    fn greet(&self) -> String;
}

struct Hello {
    name: String,
}

impl Greeter for Hello {
    fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

struct Goodbye {
    name: String,
}

impl Greeter for Goodbye {
    fn greet(&self) -> String {
        format!("Goodbye, {}!", self.name)
    }
}

fn create_hello(name: &str) -> Hello {
    Hello {
        name: name.to_string(),
    }
}

fn call_greet(g: &dyn Greeter) -> String {
    g.greet()
}

fn helper() -> String {
    let h = create_hello("world");
    call_greet(&h)
}

fn main() {
    let result = helper();
    println!("{}", result);
}

trait Chain {
    fn a(&self) -> String;
    fn b(&self) -> String;
}

impl Chain for Hello {
    fn a(&self) -> String {
        <Hello as Chain>::b(self)
    }

    fn b(&self) -> String {
        self.greet()
    }
}

trait Marker {}

impl Marker for Goodbye {}
