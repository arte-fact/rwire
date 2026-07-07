fn main() {
    let greeting = "hello from the sample couocu hello";
    for (i, word) in greeting.split(' ').enumerate() {
        println!("{i}: {word}");
    }
}
