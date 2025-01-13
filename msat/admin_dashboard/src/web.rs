use shared_components::types::*;
use std::collections::BTreeMap;

//  subject,classroom,class
type LessonData = (String, String, String);
// week day and class_id
type Current = Option<(u8, u8)>;
// Place
type DutyData = (u8, String);
type Hour = Orb<LessonData, Orb<DutyData, ()>>;

// Key: (week_day, lesson num), Value: (class, classroom, subject) 
type Lessons = BTreeMap<(u8, u8), ((u16, u16), String, String, String)>;

pub fn dashboard
(name: &str, current: Current, hour: Hour, lang: Language, lessons: Lessons) -> String{
    let mut max_lesson = 0;
    for (_, lesson_num) in lessons.keys(){
        if max_lesson < *lesson_num{
            max_lesson = *lesson_num;
        }
    }
    let mut to_ret = "".to_string();

    let currently = match current{
        Some((current_weekd, current_lessonn)) => (current_weekd, current_lessonn),
        None => (0, 0)
    };

    for weekd in 1..=5{
        to_ret.push_str(&format!("<weekd><p>{}</p><lessons-wd>", crate::weekd_to_string(&lang, weekd)));
        for lesson_numb in 1..=max_lesson{
            if currently == (weekd, lesson_numb){
                to_ret.push_str("<lesson class=\"current_lesson\">");
            }
            else{
                to_ret.push_str("<lesson>")
            }
            if let Some(((lesson_start, lesson_end),subject, class, classroom)) = lessons.get(&(weekd, lesson_numb)){
                to_ret.push_str(
                    &format!("<p><strong>{} {}</strong></p><p>{}</p><p>{}</p><p>{}</p><p>{}</p>",
                        lang.english_or("Lesson", "Lekcja"),
                        lesson_numb,
                        (*lesson_start, *lesson_end).msat_to_string(),
                        subject,
                        class, 
                        classroom
                    )
                );
            }
            else{
                to_ret.push_str(&format!("<p><strong>.</strong><p>.</p></p><p>{}</p><p>.</p><p>.</p>", lang.english_or("No Data", "Brak danych")));
            }
            to_ret.push_str("   </lesson>")
        }
        to_ret.push_str("</lessons-wd></weekd>");
    }
    format!
    (
    "
    <h1>{} {}!</h1>
    <h2>{}</h2>
    <h3>{}</h3>
    <lessons>
       {} 
    </lessons>
    <h3>{}</h3>
    <duties>
    </duties>
    ",
    lang.english_or("Welcome Back,", "Witaj spowrotem"), name,
    match hour{
        Orb::Data((subject, classroom, class)) => {
            lang.english_or(
                &format!("You have <italic>{}</italic> with <italic>{}</italic> in <italic>{}</italic>",
                    subject, class, classroom
                ),
                &format!("Masz teraz <italic>{}</italic> z <italic>{}</italic> w <italic>{}</italic>", 
                    subject, class, classroom
                )
            )
        },
        Orb::Alt(duty_orb) => {
            match duty_orb{
                Orb::Data((lesson_num, place)) => {
                    lang.english_or(
                        &format!("You have duty in {} after lesson {}!", place, lesson_num), 
                        &format!("Masz dyżur w {} po lekcji {}!", place, lesson_num)
                    )
                }
                Orb::Alt(_) => {
                    lang.english_or("You <strong>don't have</strong> Duty, enjoy break!", 
                        "<strong>Nie masz</strong> dyżuru, ciesz się przerwą!")
                }
            }
        }
    },
    lang.english_or("Lessons", "Lekcje"),
    to_ret,
    lang.english_or("Duties", "Dyżury")
    )
}

pub fn login(lang: Language) -> String{
    return format!(
        "
        <div class='login'>
        <h1>{}</h1>
        <col>
            <input id='id' type=number max=65536 min=1 placeholder='{}'>
            <input id='password' type=password placeholder='{}'>
            <button id='submit_but'>{}</button>
        </col>
        </div>",
        lang.english_or("Log in", "Zaloguj się"),
        lang.english_or("Enter your teacher ID", "Wstaw twój Identyfikator"),
        lang.english_or("Enter your password"  , "Wstaw hasło"),
        lang.english_or("Login", "Zaloguj się")
    );
}

pub fn post_login(lang: &Language) -> String{
    return format!(
        "
        <div class='login'>
            <h1>{}</h1>
            <h2>{}</h2>
        </div>
        ",
        lang.english_or("Login succesfully", "Zalogowano się pomyślnie"),
        lang.english_or("You can continue", "Możesz kontynuować")
    );
}

pub fn admin_init(lang: &Language) -> String{
    return format!(
        "
        <div>
        <h1>{}</h1>
        <select name='method' id='select'>
            <option value='GET+1'>{}</option>
            <option value='GET+2'>{}</option>
            <option value='GET+3'>{}</option>
            <option value='GET+4'>{}</option>
            <option value='GET+5'>{}</option>
            <option value='GET+6'>{}</option>
            <option value='GET+7'>{}</option>
        </select>
        <button id='submit'>{}</button>
        <div id='DATABASE-CONTENT'>

        </div>
        </div>
        ",
        lang.english_or("Admin Dashboard", "Panel Administratora"),
        lang.english_or("GET data about lessons (class-view)", "Pobierz dane o lekcjach (widok klas)"),
        lang.english_or("GET data about teachers", "Pobierz dane o nauczycielach"),
        lang.english_or("GET data about Duties", "Pobierz dane o dyżurach"),
        lang.english_or("GET data about Subjects", "Pobierz dane o przedmiotach"),
        lang.english_or("GET data about Classrooms", "Pobierz dane o klasach (pomieszczenia)"),
        lang.english_or("GET data about lesson hours", "Pobierz dane o godzinach lekcyjnych"),
        lang.english_or("GET data about break hours", "Pobierz dane o godzinach przerw"),
        lang.english_or("Send Request", "Wyślij Zapytanie")
    );
}
