fn main() {
    let greeting = "hello from the sample couocu";
    for (i, word) in greeting.split(' ').enumerate() {
        println!("{i}: {word}");
    }
}
