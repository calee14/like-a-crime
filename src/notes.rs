use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref CHORD_DATABASE: HashMap<&'static str, Vec<&'static str>> = {
    let mut chords = HashMap::new();

    // major chords
    chords.insert("C", vec!["C", "E", "G"]);
    chords.insert("C#", vec!["C#", "F", "G#"]);
    chords.insert("D", vec!["D", "F#", "A"]);
    chords.insert("D#", vec!["D#", "G", "A#"]);
    chords.insert("E", vec!["E", "G#", "B"]);
    chords.insert("F", vec!["F", "A", "C"]);
    chords.insert("F#", vec!["F#", "A#", "C#"]);
    chords.insert("G", vec!["G", "B", "D"]);
    chords.insert("G#", vec!["G#", "C", "D#"]);
    chords.insert("A", vec!["A", "C#", "E"]);
    chords.insert("A#", vec!["A#", "D", "F"]);
    chords.insert("B", vec!["B", "D#", "F#"]);

    // minor chords
    chords.insert("Cm", vec!["C", "D#", "G"]);
    chords.insert("C#m", vec!["C#", "E", "G#"]);
    chords.insert("Dm", vec!["D", "F", "A"]);
    chords.insert("D#m", vec!["D#", "F#", "A#"]);
    chords.insert("Em", vec!["E", "G", "B"]);
    chords.insert("Fm", vec!["F", "G#", "C"]);
    chords.insert("F#m", vec!["F#", "A", "C#"]);
    chords.insert("Gm", vec!["G", "A#", "D"]);
    chords.insert("G#m", vec!["G#", "B", "D#"]);
    chords.insert("Am", vec!["A", "C", "E"]);
    chords.insert("A#m", vec!["A#", "C#", "F"]);
    chords.insert("Bm", vec!["B", "D", "F#"]);

    chords
    };
}

// takes in top frequencies from a window and searches for chord
// otherwise it will choose the dominant frequency
pub fn frequency_to_note(frequencies: &[f32]) -> String {
    if frequencies.is_empty() {
        return "N/A".to_string();
    }

    let mut detected_notes: Vec<String> = Vec::new();
    for &frequency in frequencies {
        if frequency < 20.0 {
            continue;
        }
        let midi_note = 69.0 + 12.0 * (frequency / 440.0).log2();
        let rounded_midi = midi_note.round() as i32;

        let note_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let note_index = (rounded_midi % 12) as usize;

        detected_notes.push(note_names[note_index].to_string());
    }

    if detected_notes.is_empty() {
        return "N/A".to_string();
    }

    detected_notes.dedup();

    for (chord_name, chord_notes) in CHORD_DATABASE.iter() {
        let chord_match = chord_notes
            .iter()
            .all(|&chord_note| detected_notes.iter().any(|detected| detected == chord_note));

        if chord_match {
            return format!("{} chord", chord_name);
        }
    }

    // A4 = 440Hz = MIDI note 69
    let midi_note = 69.0 + 12.0 * (frequencies[0] / 440.0).log2();
    let rounded_midi = midi_note.round() as i32;

    let note_names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (rounded_midi / 12) - 1;
    let note_index = (rounded_midi % 12) as usize;

    let cents_off = ((midi_note - rounded_midi as f32) * 100.0).round() as i32;

    format!("{}{} ({:+}¢)", note_names[note_index], octave, cents_off)
}
