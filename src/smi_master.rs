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

//use embedded_time::{fixed_point::FixedPoint,};

//use hal::pio::{PIOBuilder, 
//   ShiftDirection, 
//    Tx, 
//    UninitStateMachine, 
//    PIO, 
//    SM0};
//use hal::gpio::bank0::Gpio5;
//use hal::gpio::{Function, Pin, PinId};
//use pac::PIO0;

//
//use PioDriver::PioDriver;

//impl PioDriver for smi_master{
 //   fn init(&self, 
 //       mut sm: PioStateMachineInstance<Pio0, Sm0>, 
 //       mdio_pin: ,
 //       mdc_pin: AnyPin,
  //      freq: RateExtU32 ) {
  //      // Setup 
//        let prg = pio_proc::pio_asm!(
 //           ".wrap_target
 //           .set pins 0
 //       start:
///            pull block  
  //         set pindirs 1
 //           set x, 31
  //         jmp preable             side 0
 //       preamble:
 //           set pins 1              side 1 [4]
 //           set pins 1              side 0 [2]
 //           jmp x-- preamble        side 0 [2]
 //           set pins 0              side 1 [4]
  //          nop                     side 0 [2]
 //           set y 11                       [2] 
 //           set pins 1              side 1 [4]
  //          nop                     side 0 [1]
  //      addr:
   //         set x 15                       [3]
  //          out pins 1              side 1 [4]
  //          jmp y-- addr            side 0 [1]  
 //           nop                            [3]
 //           nop                     side 1 [4]
 //           nop                     side 0 [4]
 //           nop                     side 1 [4]
 //           jmp osre! write_data    side 0 [2]
 //       read:
 //           set pindirs 0           side 0 [2]
  //      read_data:
 //           in isr 1                side 1 [4]
 //           jmp x-- read_data       side 0 [4]
 //           jmp start               side 0 [1]
 //       write_data:
 //           nop                     side 0 [1]
 //           out pins 1              side 1 [4]
 //           jmp x-- write_data      side 0 [3]
 //           .wrap"
  //      );
 //   }
//}

