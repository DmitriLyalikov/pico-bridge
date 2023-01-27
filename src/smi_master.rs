// SMI Master 
// -----------------------------------------------------------------------------
//
// This state machine implements SMI/MDIO master interface with the use of sidesetting 
// to synchronize the MDC clock 
//
// This state machine is blocking until TX FIFO is not empty
// 
// SMI Transaction: | Preamble   | SOF | OpCode | PhyAddr | Reg Addr | TA | Data |
//                    32 bit       2     2         5        5          2    16
//   Pin assignments:
// - MDC is side-set pin 0
// - MDIO is pin 0

.program smi_maaster

.side_set 1 opt 

;jmp entry_point
;entry_point:
;   set x, 31
    pull block        ; stall until TX FIFO is full
;    jmp preamble side 0
;    jmp sof side 0
     mov x pins  side 1 ; set first bit opcode[0]
;    jmp x-- write side 0; jump to write if first bit of opcode is 1
;    jmp read    side 0; jump to read if first but of opcode is 0
;    jmp entry_point
;preamble:             ; Preamble: Stream of 32 high bits)
;    set pins 1 side 1 
;    jmp x-- preamble side 0
;sof:        ; Start of Frame: (01)
;    set pins 0 side 1 
;    out x 1 side 0    ; first bit of OSRE (OPcode[0]). If
 ;   set pins 1 side 1
;    set y 11 side 0   ; next 11 bits Op[1] PHY[0-4] REG[0-4]
;read:      
    mov y pins side 1
    jmp y-- read side 0
    set pindirs 0 side 1 ; set pin to input, Next two clock cycles are TA bits
    nop side 0
    set y 15 side 1
    jmp get_data side 0
;write:    
    out pins 1 side 1
    jmp y-- write side 0
    nop side 1       ; Next two clock cycles are TA bits
    nop side 0
    set y 15 side 1
    jmp data side 0    
;write_data:
    out pins 1 side 1
    jmp y-- write_data side 0 
;read_data:
 
    in pins 1 side 1
    jmp y-- read_data side 0

.program smi_master
  
  public entry_point:
  pull
  set x, 23 ; Loop over 24 bits
  bitloop:
  set pins, 1 ; Drive pin high
  out y, 1 [5] ; Shift 1 bit out, and write it to y
  jmp !y skip ; Skip the extra delay if the bit was 
  nop [5]
 skip:
 set pins, 0 [5]
 jmp x-- bitloop ; Jump if x nonzero, and decrement 
 jmp entry_point

% c-sdk {
static inline void pwm_program_init(PIO pio, uint sm, uint offset, uint pin) {
   pio_gpio_init(pio, pin);
   pio_sm_set_consecutive_pindirs(pio, sm, pin, 1, true);
   pio_sm_config c = pwm_program_get_default_config(offset);
   sm_config_set_sideset_pins(&c, pin);
   pio_sm_init(pio, sm, offset, &c);
}
%}