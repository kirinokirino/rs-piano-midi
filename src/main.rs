use midly::num::{u15, u24};
use midly::Timing::Metrical;
use midly::{MidiMessage, Smf};

const MIDI_FILE: &'static [u8] = include_bytes!("../assets/NightOfNights.mid");

fn main() {
    let s = Song::new();
}

#[derive(Debug, Clone)]
struct Song {
    notes: Vec<MidiMessage>,
    time_signature: TimeSignature,
}

impl Song {
    pub fn new() -> Self {
        // Smf = Standard Midi File
        let smf = Smf::parse(MIDI_FILE).unwrap();
        println!("Parsed! header: {:?}", smf.header);
        // Header { format: SingleTrack, timing: Metrical(u15(384)) }
        let ticks_per_beat = if let Metrical(tpb) = smf.header.timing {
            tpb
        } else {
            u15::new(0)
        };

        let mut notes: Vec<MidiMessage> = Vec::new();
        let mut time_signature = None;
        let mut microseconds_per_beat = None;
        let mut _track_name = None;
        for (_track_id, track) in smf.tracks.iter().enumerate() {
            for (event_id, event) in track.iter().enumerate() {
                match event.kind {
                    midly::TrackEventKind::Midi {
                        channel: _,
                        message,
                    } => {
                        notes.push(message);
                    }
                    midly::TrackEventKind::Meta(variant) => match variant {
                        midly::MetaMessage::TrackNumber(_) => todo!(),
                        midly::MetaMessage::Text(_) => todo!(),
                        midly::MetaMessage::Copyright(_) => todo!(),
                        midly::MetaMessage::TrackName(name) => _track_name = Some(name),
                        midly::MetaMessage::InstrumentName(_) => todo!(),
                        midly::MetaMessage::Lyric(_) => todo!(),
                        midly::MetaMessage::Marker(_) => todo!(),
                        midly::MetaMessage::CuePoint(_) => todo!(),
                        midly::MetaMessage::ProgramName(_) => todo!(),
                        midly::MetaMessage::DeviceName(_) => todo!(),
                        midly::MetaMessage::MidiChannel(_) => todo!(),
                        midly::MetaMessage::MidiPort(_) => todo!(),
                        midly::MetaMessage::EndOfTrack => (),
                        midly::MetaMessage::Tempo(t) => microseconds_per_beat = Some(t),
                        midly::MetaMessage::SmpteOffset(_) => todo!(),
                        midly::MetaMessage::TimeSignature(a, b, c, d) => {
                            time_signature = Some((a, b, c, d));
                        }
                        midly::MetaMessage::KeySignature(_, _) => todo!(),
                        midly::MetaMessage::SequencerSpecific(_) => todo!(),
                        midly::MetaMessage::Unknown(_, _) => todo!(),
                    },
                    kind => {
                        println!("Event {event_id}: kind: {:?}, delta: {}", kind, event.delta);
                    }
                }
            }
        }
        let (numerator, denominator, clocks_per_click, _32nd_notes_per_quarter) =
            time_signature.unwrap();
        let time_signature = TimeSignature {
            numerator,
            denominator,
            clocks_per_click,
            _32nd_notes_per_quarter,
            microseconds_per_beat: microseconds_per_beat.unwrap(),
            ticks_per_beat,
        };
        Self {
            notes,
            time_signature,
        }
    }
}

#[derive(Debug, Clone)]
struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
    pub clocks_per_click: u8,
    pub _32nd_notes_per_quarter: u8,
    pub microseconds_per_beat: u24,
    pub ticks_per_beat: u15,
}
