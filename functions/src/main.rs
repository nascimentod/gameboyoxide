fn five() -> i32 {
    5
}

fn main() {
    let five = plus_one(5);

    println!("The value of five plus one is: {five}");
}

fn plus_one(x: i32) -> i32 {
    x + 1
}