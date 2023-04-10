use std::collections::HashMap;

pub struct Question {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) text: String,
    pub(crate) help: Option<String>,
    pub(crate) hints: Vec<Hint>
}

pub struct Hint {
    pub(crate) hint_title: String,
    pub(crate) hint_description: String
}

pub fn get_questions() -> HashMap<i64, Question> {
    return HashMap::from([
        (1, Question {
            name: "Dodo the Eagle [Medium]".to_string(),
            description: None,
            text: "//7 there is record implementing interface, add interface,
//methods is satisfied by field

[???]
record Eagle(double speed) implements Bird{}
public class Exercise{
  public static void main(String[] arg){
    Eagle dodo = new Eagle(3);
    Bird a1 = dodo;
    Bird a2 = new Eagle(6);
    assert a1.speed()==3;
    assert a1.flyingSpeed()==30;
    assert a2.speed()==6;
    assert a2.flyingSpeed()==60;
  }
}
        ".to_string(),
            hints: vec![],
            help: None
        })
    ]);
}