fn main() {
    let greeting = "hello from the sample";
    for (i, word) in greeting.split(' ').enumerate() {
        println!("{i}: {greeting}");
    }
}