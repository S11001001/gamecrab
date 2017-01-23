extern crate rustgirl;
use std::io::prelude::*;
use std::fs::File;
use rustgirl::register::*;
use rustgirl::opcode::*;


fn get_arg(start:usize, num:u8, res:&Vec<u8>) -> u16 {
    match num {
        3 => ((res[start + 2] as u16) << 8) + (res[start + 1] as u16) ,
        _ => 0
    }
}

fn get_cb(start:usize, y:&Vec<u8>) -> (usize, OpCode, u8) {
    let b = y[start + 1];
    match y[start + 1] {
        0x00...0x07 => (2, OpCode::RLC(lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        0x08...0x0F => (2, OpCode::RRC(lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        0x10...0x17 => (2, OpCode::RL(lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        0x18...0x1F => (2, OpCode::RR(lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        0x20...0x27 => (2, OpCode::SLA(lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        0x28...0x2F => (2, OpCode::SRA(lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        0x30...0x37 => (2, OpCode::SWAP(lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        0x38...0x3F => (2, OpCode::SRL(lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        0x40...0x7F => (2, OpCode::BIT((b - 0x40) / 8, lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        0x80...0xBF => (2, OpCode::RES((b - 0x80) / 8, lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        0xC0...0xFF => (2, OpCode::SET((b - 0xC0) / 8, lookup_mod_register(b)), 8 * lookup_mod_mult(b)),
        _ => (2, OpCode::ERR(format!("{:0>2X}", y[start])), 0)
    }
}

fn lookup_mod_register(b:u8) -> Register {
  let registers = [Register::B, Register::C, Register::D, Register::E, Register::H, Register::L, Register::HL_ADDR, Register::A];
  registers[(b % 8) as usize]
}

fn lookup_mod_cycles(b:u8) -> u8 {
  if (b % 8) == 6 { 8 } else { 4 }
}

fn lookup_mod_mult(b:u8) -> u8 {
  if (b % 8) == 6 { 2 } else { 1 }
}

fn lookup_LD_R(start:usize, b:u8) -> (usize, OpCode, u8){
  let idx = b - 0x40;
  let registers = [Register::B, Register::C, Register::D, Register::E, Register::H, Register::L, Register::HL_ADDR, Register::A];
  let (left, right) = (registers[(idx/8) as usize], lookup_mod_register(b));
  let cycles = if (idx / 8) == 6 || (idx % 8) == 6 { 8 } else { 4 };
  (1, OpCode::LD_R(left, right), cycles)
}

fn lookup_mod_op_a(op:fn(Register, Register) -> OpCode, b:u8) -> (usize, OpCode, u8) {
    (1, op(Register::A, lookup_mod_register(b)), lookup_mod_cycles(b))
}

fn lookup_mod_op(op:fn(Register) -> OpCode, b:u8) -> (usize, OpCode, u8) {
    (1, op(lookup_mod_register(b)), lookup_mod_cycles(b))
}

fn lookup_op(start:usize, y:&Vec<u8>) -> (usize, OpCode, u8) {
    let res = match y[start] {
        0x00 => (1, OpCode::NOP, 4),
        0x10 => (2, OpCode::STOP, 4),
        0x20 => (2, OpCode::JR_C(Cond::NZ, y[start + 1] as i8), 12), // 12/8 The first arg should be a signed byte
        0x30 => (2, OpCode::JR_C(Cond::NC, y[start + 1] as i8), 12), //12/8 The first arg should be a signed byte
        0x01 => (3, OpCode::LD_M(Register::BC, get_arg(start, 3, y)), 12),
        0x11 => (3, OpCode::LD_M(Register::DE, get_arg(start, 3, y)), 12),
        0x21 => (3, OpCode::LD_M(Register::HL, get_arg(start, 3, y)), 12),
        0x31 => (3, OpCode::LD_M(Register::SP, get_arg(start, 3, y)), 12),
        0x02 => (1, OpCode::LD_R(Register::BC, Register::A), 8),
        0x12 => (1, OpCode::LD_R(Register::DE, Register::A), 8),
        0x22 => (1, OpCode::LD_R(Register::HLP, Register::A), 8),
        0x32 => (1, OpCode::LD_R(Register::HLM, Register::A), 8),
        0x03 => (1, OpCode::INC(Register::BC), 8),
        0x13 => (1, OpCode::INC(Register::DE), 8),
        0x23 => (1, OpCode::INC(Register::HL), 8),
        0x33 => (1, OpCode::INC(Register::SP), 8),
        0x04 => (1, OpCode::INC_F(Register::B), 4),
        0x14 => (1, OpCode::INC_F(Register::D), 4),
        0x24 => (1, OpCode::INC_F(Register::H), 4),
        0x34 => (1, OpCode::INC_F(Register::HL), 12),
        0x05 => (1, OpCode::DEC_F(Register::B), 4),
        0x15 => (1, OpCode::DEC_F(Register::D), 4),
        0x25 => (1, OpCode::DEC_F(Register::H), 4),
        0x35 => (1, OpCode::DEC_F(Register::HL), 12),
        0x06 => (2, OpCode::LD(Register::B, y[start + 1]), 8),
        0x16 => (2, OpCode::LD(Register::D, y[start + 1]), 8),
        0x26 => (2, OpCode::LD(Register::H, y[start + 1]), 8),
        0x36 => (2, OpCode::LD(Register::HL_ADDR, y[start + 1]), 12),
        0x07 => (1, OpCode::RLCA, 4),
        0x17 => (1, OpCode::RLA, 4),
        0x27 => (1, OpCode::DAA, 4),
        0x37 => (1, OpCode::SCF, 4),
        0x08 => (3, OpCode::LD_R(Register::ADDR(get_arg(start, 3, y)), Register::SP), 20),
        0x18 => (2, OpCode::JR(y[start + 1] as i8), 4),
        0x28 => (2, OpCode::JR_C(Cond::Z, y[start + 1] as i8), 4), // 12/8
        0x38 => (2, OpCode::JR_C(Cond::C, y[start + 1] as i8), 4),
        0x09 => (1, OpCode::ADD(Register::HL, Register::BC), 8),
        0x19 => (1, OpCode::ADD(Register::HL, Register::DE), 8),
        0x29 => (1, OpCode::ADD(Register::HL, Register::HL), 8),
        0x39 => (1, OpCode::ADD(Register::HL, Register::SP), 8),
        0x0B => (1, OpCode::DEC(Register::BC), 8),
        0x1B => (1, OpCode::DEC(Register::DE), 8),
        0x2B => (1, OpCode::DEC(Register::HL), 8),
        0x3B => (1, OpCode::DEC(Register::SP), 8),
        0x0C => (1, OpCode::INC_F(Register::C), 4),
        0x1C => (1, OpCode::INC_F(Register::E), 4),
        0x2C => (1, OpCode::INC_F(Register::L), 4),
        0x3C => (1, OpCode::INC_F(Register::A), 4),
        0x0D => (1, OpCode::DEC_F(Register::C), 4),
        0x1D => (1, OpCode::DEC_F(Register::E), 4),
        0x2D => (1, OpCode::DEC_F(Register::L), 4),
        0x3D => (1, OpCode::DEC_F(Register::A), 4),
        0x0E => (2, OpCode::LD(Register::C, y[start + 1]), 8),
        0x1E => (2, OpCode::LD(Register::E, y[start + 1]), 8),
        0x2E => (2, OpCode::LD(Register::L, y[start + 1]), 8),
        0x3E => (2, OpCode::LD(Register::A, y[start + 1]), 8),
        0x0F => (1, OpCode::RRCA, 4),
        0x1F => (1, OpCode::RRA, 4),
        0x2F => (1, OpCode::CPL, 4),
        0x3F => (1, OpCode::CCF, 4),
        0x0A => (1, OpCode::LD_R(Register::A, Register::BC), 8),
        0x1A => (1, OpCode::LD_R(Register::A, Register::DE), 8),
        0x2A => (1, OpCode::LD_R(Register::A, Register::HLP), 8),
        0x3A => (1, OpCode::LD_R(Register::A, Register::HLM), 8),
        0xE0 => (2, OpCode::LD_R(Register::ADDR(0xFF00 + (y[start + 1] as u16)), Register::A), 12),
        0xF0 => (2, OpCode::LD_R(Register::A, Register::ADDR(0xFF00 + (y[start + 1] as u16))), 12),
        0xC2 => (3, OpCode::JP_C(Cond::NZ, get_arg(start, 3, y)), 16), // 16/12
        0xD2 => (3, OpCode::JP_C(Cond::NC, get_arg(start, 3, y)), 16), // 16/12
        0xE2 => (1, OpCode::LD_R(Register::CH, Register::A), 8),
        0xF2 => (1, OpCode::LD_R(Register::A, Register::CH), 8),
        0xC3 => (3, OpCode::JP(get_arg(start, 3, y)), 16),
        0xF3 => (1, OpCode::DI, 4),
        0xC4 => (3, OpCode::CALL_C(Cond::NZ, Register::ADDR(get_arg(start, 3, y))), 24), // 24/12
        0xD4 => (3, OpCode::CALL_C(Cond::NC, Register::ADDR(get_arg(start, 3, y))), 24), // 24/12
        0xCB => get_cb(start, y),
        0xFB => (1, OpCode::EI, 4),
        0xCC => (3, OpCode::CALL_C(Cond::Z, Register::ADDR(get_arg(start, 3, y))), 24), // 24/12
        0xDC => (3, OpCode::CALL_C(Cond::C, Register::ADDR(get_arg(start, 3, y))), 24), // 24/12
        0xCD => (3, OpCode::CALL(Register::ADDR(get_arg(start, 3, y))), 24),
        0x76 => (1, OpCode::HALT, 4),
        b @ 0x40...0x7F => lookup_LD_R(start, b), //All the registers that use HL have the wrong cycle count
        b @ 0x80...0x88 => lookup_mod_op_a(OpCode::ADD, b),
        b @ 0x88...0x8F => lookup_mod_op_a(OpCode::ADD_C, b),
        b @ 0x90...0x98 => lookup_mod_op(OpCode::SUB, b),
        b @ 0x98...0x9F => lookup_mod_op_a(OpCode::SUB_C, b),
        b @ 0xA0...0xA8 => lookup_mod_op(OpCode::AND, b),
        b @ 0xA8...0xAF => lookup_mod_op(OpCode::XOR, b),
        b @ 0xB0...0xB8 => lookup_mod_op(OpCode::OR, b),
        b @ 0xB8...0xBF => lookup_mod_op(OpCode::CP, b),
        0xC0 => (1, OpCode::RET_C(Cond::NZ), 8), // actually 20/8
        0xD0 => (1, OpCode::RET_C(Cond::NC), 8), // actually 20/8
        0xC1 => (1, OpCode::POP(Register::BC), 12),
        0xD1 => (1, OpCode::POP(Register::DE), 12),
        0xE1 => (1, OpCode::POP(Register::HL), 12),
        0xF1 => (1, OpCode::POP(Register::AF), 12),
        0xC5 => (1, OpCode::PUSH(Register::BC), 16),
        0xD5 => (1, OpCode::PUSH(Register::DE), 16),
        0xE5 => (1, OpCode::PUSH(Register::HL), 16),
        0xF5 => (1, OpCode::PUSH(Register::AF), 16),
        0xC6 => (2, OpCode::ADD_d8(Register::A, y[start + 1]), 8),
        0xD6 => (2, OpCode::SUB_d8(y[start + 1]), 8),
        0xE6 => (2, OpCode::AND_d8(y[start + 1]), 8),
        0xF6 => (2, OpCode::OR_d8(y[start + 1]), 8),
        0xC7 => (1, OpCode::RST(0x00), 8),
        0xD7 => (1, OpCode::RST(0x10), 16),
        0xE7 => (1, OpCode::RST(0x20), 16),
        0xF7 => (1, OpCode::RST(0x30), 16),
        0xC8 => (1, OpCode::RET_C(Cond::Z), 8), // actually 20/8
        0xD8 => (1, OpCode::RET_C(Cond::C), 8), // actually 20/8
        0xE8 => (2, OpCode::ADD_r8(Register::SP, y[start + 1] as i8), 16),
        0xF8 => (2, OpCode::LD_R(Register::HL, Register::SP_OFF(y[start + 1] as i8)), 12),
        0xC9 => (1, OpCode::RET, 16),
        0xD9 => (1, OpCode::RETI, 16),
        0xE9 => (1, OpCode::JP_HL, 4),
        0xF9 => (1, OpCode::LD_R(Register::SP, Register::HL), 8),
        0xCA => (3, OpCode::JP_C(Cond::Z, get_arg(start, 3, y)), 16), // 16/12
        0xDA => (3, OpCode::JP_C(Cond::C, get_arg(start, 3, y)), 16),
        0xEA => (3, OpCode::LD_R(Register::ADDR(get_arg(start, 3, y)), Register::A), 16),
        0xFA => (3, OpCode::LD_R(Register::A, Register::ADDR(get_arg(start, 3, y))), 16),
        0xCE => (2, OpCode::ADD_C_d8(Register::A, y[start + 1]), 8),
        0xDE => (2, OpCode::SUB_C_d8(Register::A, y[start + 1]), 8),
        0xEE => (2, OpCode::XOR_d8(y[start + 1]), 8),
        0xFE => (2, OpCode::CP_d8(y[start + 1]), 8),
        0xCF => (1, OpCode::RST(0x08), 16),
        0xDF => (1, OpCode::RST(0x18), 16),
        0xEF => (1, OpCode::RST(0x28), 16),
        0xFF => (1, OpCode::RST(0x38), 16),
        _ => (1, OpCode::ERR(format!("{:0>2X}", y[start])), 0)
    };
    res
}

fn exec_ld_m(reg: Register, val: u16, curr_addr: usize, cpu: &mut Cpu) -> usize {
    use rustgirl::register::Register::*;
    match reg {
        _ => { write_multi_register(reg, val, cpu); curr_addr }
    }
}

fn exec_ld(reg: Register, val: u8, curr_addr: usize, cpu: &mut Cpu) -> usize {
    use rustgirl::register::Register::*;
    match reg {
        _ => { write_register(reg, val, cpu); curr_addr }
    }
}

fn exec_xor(reg: Register, cpu: &mut Cpu) -> () {
    use rustgirl::register::Register::*;
    let reg_a_val = read_register(A, cpu);
    let reg_val = read_register(reg, cpu);
    let res = reg_a_val^reg_val;
    let res_f = (if res == 0 { 1 } else { 0 }) << 7;

    match reg {
        _ => { 
            write_register(A, res, cpu);
            write_register(F, res_f, cpu);
        }
    }
}


fn exec_instr(op: OpCode, curr_addr: usize, cpu: &mut Cpu) -> usize {
    use rustgirl::register::Register::*;
    use rustgirl::opcode::OpCode::*;
    match op {
        JP(addr) => addr as usize,
        JP_HL => read_multi_register(Register::HL, cpu) as usize,
        NOP => curr_addr,
        XOR(reg) => { exec_xor(reg, cpu); curr_addr },
        LD(reg, val) => exec_ld(reg, val, curr_addr, cpu),
        LD_M(reg, val) => exec_ld_m(reg, val, curr_addr, cpu),
        _ => unreachable!()
    }
}

fn write_multi_register(reg: Register, val: u16, cpu: &mut Cpu) -> () {
   use rustgirl::register::Register::*;
   let (l_byte, r_byte) = ((val >> 8) as u8, (0x0F & val) as u8);
   match reg {
       HL => { cpu.H = l_byte; cpu.L = r_byte; },
       AF => { cpu.A = l_byte; cpu.F = r_byte; },
       BC => { cpu.B = l_byte; cpu.C = r_byte; },
       DE => { cpu.D = l_byte; cpu.E = r_byte; },
       SP => cpu.SP = val,
       _ => unreachable!()
   };
}

fn read_register(reg: Register, cpu: &mut Cpu) -> u8 {
   use rustgirl::register::Register::*;
   match reg {
       A => cpu.A,
       B => cpu.B,
       C => cpu.C,
       D => cpu.D,
       E => cpu.E,
       F => cpu.F,
       H => cpu.H,
       L => cpu.L,
       _ => unreachable!()
   } 
}

fn write_register(reg: Register, val: u8, cpu: &mut Cpu) -> () {
   use rustgirl::register::Register::*;
   match reg {
       A => cpu.A = val,
       B => cpu.B = val,
       C => cpu.C = val,
       D => cpu.D = val,
       E => cpu.E = val,
       F => cpu.F = val,
       H => cpu.H = val,
       L => cpu.L = val,
       _ => unreachable!()
   } 
}

fn read_multi_register(reg: Register, cpu: &mut Cpu) -> u16 {
   use rustgirl::register::Register::*;
   match reg {
       HL => ((cpu.H as u16) << 8)  + (cpu.L as u16),
       AF => ((cpu.A as u16) << 8)  + (cpu.F as u16),
       BC => ((cpu.B as u16) << 8)  + (cpu.C as u16),
       DE => ((cpu.D as u16) << 8)  + (cpu.E as u16),
       SP => cpu.SP,
       PC => cpu.PC,
       _ => unreachable!()
   } 
}

struct Cpu {
    A: u8,
    B: u8,
    C: u8,
    D: u8,
    E: u8,
    F: u8,
    H: u8,
    L: u8,
    SP: u16,
    PC: u16
}

fn main() {
    // Representing A, F, B, C, D, E, H, L in that order
    let mut cpu = Cpu { A: 0, B: 0, C:0, D:0, E:0, F:0, H:0, L:0, SP:0, PC: 0};
    let mut f = File::open("DMG_ROM.bin").unwrap();
//    let mut f = File::open("kirby.gb").unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).ok();
    let mut next_addr = 0;
    for _ in 1..256 {
        let (op_length, instr, cycles) = lookup_op(next_addr, &buffer);
        println!("Address {:4>0X}: {:?}", next_addr, instr);
        next_addr += op_length;
        next_addr = exec_instr(instr, next_addr, &mut cpu);
    }
}
