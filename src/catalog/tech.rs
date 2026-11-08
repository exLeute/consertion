//! Units for gamers, programmers and server people: frame rates, mouse resolution, typing
//! speed and request rates.

use super::Builder;

pub fn register(b: &mut Builder) {
    b.category("framerate", "Frame Rate & Frame Time", false);
    b.linear(&[
        ("frames per second (Hz)", "fps", "1", "fps frame rate refresh rate hz hertz monitor 144hz 240hz display game gaming"),
        ("tick rate", "tick/s", "1", "server tick rate tickrate game netcode"),
        ("frames per minute", "fpm", "1/60", ""),
    ]);
    b.reciprocal("milliseconds per frame", "ms/frame", "1000", "frame time frametime latency gaming budget");
    b.reciprocal("microseconds per frame", "µs/frame", "1e6", "frame time us");
    b.reciprocal("seconds per frame", "s/frame", "1", "render time timelapse");

    b.category("mouse", "Mouse Resolution (DPI)", false);
    b.linear(&[
        ("dots per inch (DPI / CPI)", "DPI", "1", "mouse dpi cpi counts per inch sensitivity gaming"),
        ("counts per centimeter", "c/cm", "2.54", "mouse"),
        ("counts per millimeter", "c/mm", "25.4", "mouse"),
    ]);

    b.category("typing", "Typing Speed", false);
    b.linear(&[
        ("words per minute", "WPM", "5", "typing speed wpm keyboard test"),
        ("characters per minute", "CPM", "1", "typing speed keystrokes kpm"),
        ("characters per second", "CPS", "60", "typing speed keystrokes"),
        ("words per hour", "WPH", "5/60", "transcription"),
    ]);

    b.category("eventrate", "Requests & Events per Time", false);
    b.linear(&[
        ("per second", "/s", "1", "rps qps requests per second queries events throughput api rate limit"),
        ("per minute", "/min", "1/60", "rpm requests per minute rate limit"),
        ("per hour", "/h", "1/3600", "requests per hour rate limit"),
        ("per day", "/day", "1/day", "requests per day daily active"),
        ("per week", "/week", "1/(7*day)", "weekly"),
        ("per month", "/month", "12/yr", "requests per month monthly mau"),
        ("per year", "/year", "1/yr", "annual"),
        ("per millisecond", "/ms", "1000", "high frequency"),
    ]);
}
