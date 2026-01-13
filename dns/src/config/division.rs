pub fn trigger_division(b: i32) {
    let a: i32 = 100;

    //SINK
    let (result, _overflow) = a.overflowing_div_euclid(b);

    let _ = result;
}
