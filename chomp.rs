pub use std::str::{Chars};
pub use std::iter::{Enumerate};

#[deriving(Show)]
pub struct ChompResult<'cr> {
    pub value: &'cr str,
    pub hitEof: bool,
    pub span: Span
}

impl<'cri> ChompResult<'cri> {
    pub fn combine(&mut self, target: ChompResult, code: &'cri str) -> ChompResult<'cri> {
        if self.span.startPos.index >= target.span.startPos.index {
            fail!("The second ChompResult does not start immediately after the first one.");
        }

        ChompResult { span: Span { startPos: self.span.startPos, endPos: target.span.endPos },
                      hitEof: target.hitEof,
                      value: code.slice(self.span.startPos.index, target.span.endPos.index) }
    }
}

#[deriving(Show)]
#[deriving(PartialEq)]
pub struct Position {
    pub index: uint,
    pub lineNo: uint,
    pub colNo: uint
}

#[deriving(Show)]
#[deriving(PartialEq)]
pub struct Span {
    pub startPos: Position,
    pub endPos: Position
}

pub struct Chomper<'chomper> {
    pub code: &'chomper str,
    pub index: uint,
    char_iterator: Enumerate<Chars<'chomper>>,
    pub isEof: bool,
    pub lineNo: uint,
    pub colNo: uint,
}

impl<'ci> Chomper<'ci> {
    pub fn new(code: &'ci str) -> Chomper<'ci> {
        // don't forget, line numbers start at 1!!!!
        Chomper{code: code, index: 0, char_iterator: code.chars().enumerate(), isEof: false, lineNo: 1, colNo: 0}
    }

    pub fn position(&self) -> Position {
        Position { index: self.index, lineNo: self.lineNo, colNo: self.colNo }
    }

    fn assert_not_eof(&self) {
        if self.isEof {fail!("Chomper is at EOF."); }
    }

    pub fn peek(&self) -> Option<char> {
        let target = self.index;
        if target >= self.code.len() {return None};
        Some(self.code.char_at(target))
    }

    pub fn text(&self) -> &'ci str {
        self.code.slice_from(self.index)
    }

    pub fn next(&mut self) -> Option<(uint, char)> {
        self.assert_not_eof();
        let result = self.char_iterator.next();
        self.index = self.index + 1;

        match result {
            None => {
                self.isEof = true;
            },
            Some((_, '\n')) => {
                self.lineNo = self.lineNo + 1;
                self.colNo = 0;
            },
            _ => self.colNo = self.colNo + 1
        };

        return result;
    }

    pub fn expect(&mut self, expectation: &str) -> ChompResult<'ci> {
        if ! self.text().starts_with(expectation) {
            fail!("At index {}, expected {} but got \r\n {}.", self.index, expectation, self.text())
        }

        // let mut chomped = 0;

        // self.chomp(|_| {
        //     chomped = chomped + 1;
        //     chomped > expectation.len()
        // }).unwrap()

        self.chomp_count(expectation.len()).unwrap()
    }

    pub fn chomp_count(&mut self, count: uint) -> Option<ChompResult<'ci>> {
        let mut chomped = 0;

        self.chomp(|_| {
            chomped = chomped + 1;
            chomped <= count
        })
    }

    pub fn chomp_till_str(&mut self, quit: |&str| -> bool) -> Option<ChompResult<'ci>> {
        self.chomp_internal(|_| false, quit)
    }

    pub fn chomp(&mut self, quit: |char| -> bool) -> Option<ChompResult<'ci>> {
        self.chomp_internal(quit, |_| false)
    }

    fn chomp_internal(&mut self, char_quit: |char| -> bool, str_quit: |&str| -> bool) -> Option<ChompResult<'ci>> {
        self.assert_not_eof();

        let mut startPosition: Option<Position> = None;
        let mut endPosition: Option<Position> = None;

        println!("starting a chomp at text: {}", self.text());
        println!("index is: {}", self.index);
        println!("isEof is {}", self.isEof);
        println!("last valid index of code is {}", self.code.len() - 1);
        // todo I KNOW this can be simplified and cleaned up
        loop {
            let should_quit = match self.peek() {
                None => {
                    // This means, there IS no next character. EOF.
                    // endIndex = Some(self.index);
                    endPosition = Some(self.position());
                    // Still need to call next(), to fully put chomper into EOF state.
                    self.next();
                    true
                },
                Some(ch) => {
                    if char_quit(ch) || str_quit(self.text()) {
                        endPosition = Some(self.position());
                        true
                    } else {
                        println!("Not time to quit yet!");
                        if startPosition == None {
                            println!("setting start index for chomp at {}", self.index);
                            startPosition = Some(self.position());
                        }
                        self.next();
                        false
                    }
                }
            };

            if should_quit {
                println!("Just about to create ChompResult");
                println!("startPosition is: {}", startPosition);
                println!("endPosition is: {}", endPosition);

                if startPosition == None {return None;}
                let cr = Some(ChompResult { value: self.code.slice(startPosition.unwrap().index, endPosition.unwrap().index),
                                            span: Span { startPos: startPosition.unwrap(), endPos: endPosition.unwrap() },
                                            hitEof: self.isEof });

                println!("Full chomp result is: {}", cr);
                return cr;
            }
        }
    }
}

#[cfg(test)]
mod test{
    use super::{Chomper};

    #[test]
    fn it_should_track_line_and_col_numbers() {
        let code = r#"This is
some text that starts at
line zero but then crosses many lines. I will
chomp it until 42, which is the first digit."#;

        let mut chomper = Chomper::new(code);
        let cr = chomper.chomp(|c| c.is_digit()).unwrap();
        assert_eq!(cr.span.startPos.lineNo, 1);
        assert_eq!(cr.span.startPos.colNo, 0);

        assert_eq!(cr.span.endPos.lineNo, 4);
        assert_eq!(cr.span.endPos.colNo, 15);
    }

    #[test]
    fn should_be_able_to_instantiate_chomper() {
        let code = "40 + 2";
        Chomper::new(code);
    }

    #[test]
    fn chomp_should_work_correctly_when_not_hitting_eof() {
        let code = "40 + 2";
        let mut chomper = Chomper::new(code);

        let result = chomper.chomp(|ch| { ! ch.is_digit() }).unwrap();

        assert_eq!(result.value, "40");
    }

    #[test]
    fn chomp_should_work_correctly_when_hitting_eof() {
        let code = "40";
        let mut chomper = Chomper::new(code);

        let result = chomper.chomp(|ch| {
            println!("Seeing if {} is a digit.", ch);
            ! ch.is_digit()
        }).unwrap();

        println!("result is: {}", result);

        assert_eq!(result.value, "40");
    }

    #[test]
    fn chomp_should_succeed_at_2_tokens_in_a_row() {
        let code = "40+2";
        let mut chomper = Chomper::new(code);

        let one = chomper.chomp(|c| ! c.is_digit()).unwrap();
        assert_eq!(one.value, "40");

        let two = chomper.chomp(|c| c != '+').unwrap();
        assert_eq!(two.value, "+");
    }

    #[test]
    #[should_fail]
    fn chomp_should_return_none_if_youre_already_at_eof_when_you_call_it() {
        let code = "40";
        let mut chomper = Chomper::new(code);

        let chomper_borrow = &mut chomper;

        let result = chomper_borrow.chomp (|_| { false}).unwrap();
        assert_eq!(result.value, "40");

        chomper_borrow.chomp(|_| { false });
    }

    #[test]
    fn expect_should_work_for_happy_path() {
        let code = "foobar";
        let mut chomper = Chomper::new(code);
        chomper.expect("foobar");
    }

    #[test]
    fn expect_multiple_times_in_a_row_happy_path_should_work() {
        let code = "foobar";
        let mut chomper = Chomper::new(code);
        chomper.expect("foo");
        chomper.expect("bar");
    }

    #[test]
    #[should_fail]
    fn expect_should_work_for_failure_path() {
        let code = "foobar";
        let mut chomper = Chomper::new(code);
        chomper.expect("fooOOPSbar");
    }

    #[test]
    fn chomp_till_str_should_work_when_there_is_a_match() {
        let code = "This is some text";
        let mut chomper = Chomper::new(code);
        let cr = chomper.chomp_till_str(|str| str.starts_with("some")).unwrap();
        println!("the cr is {}", cr);
        assert_eq!(cr.value, "This is ");
        assert_eq!(cr.span.startPos.index, 0);
        assert_eq!(cr.span.endPos.index, 8);
        assert_eq!(chomper.isEof, false);
    }

    #[test]
    fn chomp_till_str_should_work_when_there_is_no_match() {
        let code = "This is some text";
        let mut chomper = Chomper::new(code);
        let cr = chomper.chomp_till_str(|str| str.starts_with("XXXXXXX")).unwrap();
        println!("the cr is: {}", cr);
        assert_eq!(cr.value, "This is some text");
        assert_eq!(cr.span.startPos.index, 0);
        assert_eq!(cr.span.endPos.index, 17);
        assert_eq!(chomper.isEof, true);
    }

    #[test]
    fn is_empty_should_be_true_if_you_quit_chomping_immediately() {
        let code = "foobar";
        let mut chomper = Chomper::new(code);
        let cr = chomper.chomp(|c| c == 'f');
        println!("cr is {}", cr);
        assert!(cr.is_none());
    }

    #[test]
    fn is_empty_should_be_false_if_you_even_one_char_is_chomped() {
        let code = "f";
        let mut chomper = Chomper::new(code);
        let cr = chomper.chomp(|_| false).unwrap();
        println!("cr is {}", cr);
    }
}
