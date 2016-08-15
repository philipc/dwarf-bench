#![feature(test)]

extern crate test;
extern crate dwarf;
extern crate gimli;

#[bench]
fn info_dwarf(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let mut units = sections.compilation_units();
        while let Some(unit) = units.next().unwrap() {
            let abbrev = sections.abbrev(&unit.common).unwrap();
            let mut entries = unit.entries(&abbrev);
            while let Some(entry) = entries.next().unwrap() {
                test::black_box(entry.tag);
                for attribute in &entry.attributes {
                    test::black_box(attribute.at);
                    test::black_box(&attribute.data);
                }
            }
        }
    });
}

#[bench]
fn info_gimli(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let debug_info = gimli::DebugInfo::<gimli::LittleEndian>::new(&sections.debug_info);
        let debug_abbrev = gimli::DebugAbbrev::<gimli::LittleEndian>::new(&sections.debug_abbrev);
        for unit in debug_info.units() {
            let unit = unit.unwrap();
            let abbrevs = unit.abbreviations(debug_abbrev).unwrap();
            let mut cursor = unit.entries(&abbrevs);
            while cursor.next_dfs().unwrap().is_some() {
                let entry = cursor.current().unwrap();
                test::black_box(entry.tag());
                let mut attrs = entry.attrs();
                while let Some(attr) = attrs.next().unwrap() {
                    test::black_box(attr.name());
                    test::black_box(attr.value());
                }
            }
        }
    });
}

#[bench]
fn line_dwarf(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let mut r = &*sections.debug_line;
        let line_program = dwarf::line::LineNumberProgram::read(&mut r, 0, sections.endian, 8).unwrap();
        let mut lines = line_program.lines();
        while let Some(line) = lines.next().unwrap() {
            test::black_box(line);
        }
    });
}

#[bench]
fn line_gimli(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let debug_line = gimli::DebugLine::<gimli::LittleEndian>::new(&sections.debug_line);
        let header = gimli::LineNumberProgramHeader::new(debug_line, gimli::DebugLineOffset(0), 8).unwrap();
        let mut state_machine = gimli::StateMachine::new(&header);
        while let Some(row) = state_machine.next_row().unwrap() {
            test::black_box(row);
        }
    });
}
