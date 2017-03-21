use cpu::*;
use register::*;

pub enum Flag {
    Z,
    N,
    H,
    C
}

pub fn set(flag: Flag, cpu: &mut Cpu) -> () {
    write_register(Register::F, read_register(Register::F, cpu) | flag_bit(flag), cpu);
}

pub fn reset(flag: Flag, cpu: &mut Cpu) -> () {
    write_register(Register::F, read_register(Register::F, cpu) & (255 - flag_bit(flag)), cpu);
}

pub fn is_set(flag: Flag, cpu: &mut Cpu) -> bool {
    read_register(Register::F, cpu) & flag_bit(flag) != 0 
}

pub fn flag_bit(flag: Flag) -> u8 {
    use self::Flag::*;
    1 << match flag {
       Z => 7, 
       N => 6,
       H => 5,
       C => 4
    }
}
