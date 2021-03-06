use crate::net::ConnSender;
use bb_common::{net::cb, util::Chat};

#[derive(Debug)]
pub struct Scoreboard {
  conn:     ConnSender,
  position: cb::ScoreboardDisplayPosition,
  shown:    bool,
  lines:    Vec<String>,
}

impl Scoreboard {
  pub fn new(conn: ConnSender) -> Self {
    Scoreboard {
      conn,
      position: cb::ScoreboardDisplayPosition::Sidebar,
      shown: false,
      lines: vec!["".into(); 15],
    }
  }

  pub fn show(&mut self) {
    if !self.shown {
      self.conn.send(cb::packet::ScoreboardObjective {
        objective: "scoreboard".into(),
        mode:      cb::ObjectiveAction::Create {
          value: Chat::new("Scoreboard"),
          ty:    cb::ObjectiveType::Integer,
        },
      });
      self.conn.send(cb::packet::ScoreboardDisplay {
        position:  self.position,
        objective: "scoreboard".into(),
      });
      self.shown = true;
    }
  }

  pub fn hide(&mut self) {
    if !self.shown {
      self.conn.send(cb::packet::ScoreboardObjective {
        objective: "scoreboard".into(),
        mode:      cb::ObjectiveAction::Remove,
      });
      self.shown = false;
    }
  }

  pub fn display(&mut self, position: cb::ScoreboardDisplayPosition) {
    if position != self.position {
      self.position = position;
      if self.shown {
        self.conn.send(cb::packet::ScoreboardDisplay { position, objective: "scoreboard".into() });
      }
    }
  }

  pub fn clear_line(&mut self, line: u8) {
    self.conn.send(cb::packet::ScoreboardUpdate {
      username:  self.lines[line as usize].clone(),
      objective: "scoreboard".into(),
      action:    cb::ScoreboardAction::Remove,
    });
    self.lines[line as usize] = "".into();
  }
  pub fn set_line(&mut self, line: u8, text: &Chat) {
    let mut text = text.to_codes();
    if text == self.lines[line as usize] {
      return;
    }
    while self.lines.contains(&text) {
      text.push(' ');
    }
    self.conn.send(cb::packet::ScoreboardUpdate {
      username:  self.lines[line as usize].clone(),
      objective: "scoreboard".into(),
      action:    cb::ScoreboardAction::Remove,
    });
    self.lines[line as usize] = text;
    self.conn.send(cb::packet::ScoreboardUpdate {
      username:  self.lines[line as usize].clone(),
      objective: "scoreboard".into(),
      action:    cb::ScoreboardAction::Create(line.into()),
    });
  }
}
