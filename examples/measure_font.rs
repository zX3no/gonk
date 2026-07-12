fn main() {
    // Use same approach as neoui measure_text - need a font
    let data = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf").or_else(|_| std::fs::read(r"C:\Windows\Fonts\arial.ttf")).unwrap();
    let font = fontdue::Font::from_bytes(data.as_slice(), fontdue::FontSettings::default()).unwrap();
    for size in [11u32, 13] {
        let m = font.horizontal_line_metrics(size as f32).unwrap();
        println!("size {size}: new_line_size={}, ascent={}, descent={}, line_gap={}", m.new_line_size, m.ascent, m.descent, m.line_gap);
        println!("  row padtb7 => {}", (m.new_line_size).round() as i32 + 14);
        println!("  letter padt6 padb2 => {}", (m.new_line_size).round() as i32 + 8);
    }
}
