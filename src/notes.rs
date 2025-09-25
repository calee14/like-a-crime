pub fn frequency_to_note(frequency: f32) -> String {
    if frequency < 20.0 {
        return "N/A".to_string();
    }

    // A4 = 440Hz = MIDI note 69
    let midi_note = 69.0 + 12.0 * (frequency / 440.0).log2();
    let rounded_midi = midi_note.round() as i32;

    let note_names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (rounded_midi / 12) - 1;
    let note_index = (rounded_midi % 12) as usize;

    let cents_off = ((midi_note - rounded_midi as f32) * 100.0).round() as i32;

    format!("{}{} ({:+}¢)", note_names[note_index], octave, cents_off)
}
