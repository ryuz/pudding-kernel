/** 
 */


#include "arm.inc"


                .code   32
                .text
                .align  2

                .global _KERNEL_CPU_CB
                .global _kernel_interrupt_start
                .global _kernel_interrupt_end
                .global _kernel_interrupt_handler


/************************************************
  IRQ handler
 ************************************************/
                .global _kernel_exception_irq
_kernel_exception_irq:
            /* ---- レジスタ退避 */
                msr     cpsr_c, #(Mode_SYS | F_Bit | I_Bit)     /* SYSモードに移行 */
                stmfd   sp!, {r0-r3, r12, lr}                   /* SYSモードの汎用レジスタ退避 */

#ifdef _KERNEL_ARM_WITH_VFP     /* VFP利用時 */
                vpush   {d0-d7}                                 /* VFP用レジスタ退避 */
                vmrs    r1, fpscr                               /* fpscr 取り出し */
                push    {r1}                                    /* fpscr 退避 */
                vmrs    r1, fpexc                               /* fpexc 取り出し */
                push    {r1}                                    /* fpexc 退避 */
#endif

                msr     cpsr_c, #(Mode_IRQ | F_Bit | I_Bit)     /* IRQモードに移行 */
                mov     r0, lr                                  /* lr_irq 取り出し */
                mrs     r1, spsr                                /* spsr_irq 取り出し */
                msr     cpsr_c, #(Mode_SYS | F_Bit | I_Bit)     /* SYSモードに移行 */
                stmfd   sp!, {r0, r1}                           /* lr_irq, spsr_irq退避 */
                
            /* ---- 割込みマスク設定 */
                ldr     r0, =_KERNEL_CPU_CB
                ldr     r3, [r0, #CPUCB_IMSK]                   /* 古いimsk値を取り出し */
                orr     r1, r3, #I_Bit                          /* 割込みマスク */
                strb    r1, [r0, #CPUCB_IMSK]                   /* cpsr値をimsk値に設定 */
                
            /* ---- 多重割込み判定 */
                ldrb    r1, [r0, #CPUCB_INTCNT]                 /* 割り込みネストカウンタ値取得 */
                add     r1, r1, #1                              /* 割り込みネストカウンタインクリメント */
                strb    r1, [r0, #CPUCB_INTCNT]                 /* 割り込みネストカウンタ設定 */
                cmp     r1, #1
                bne     multiple_int                            /* 多重割り込みなら分岐 */
                
            /* ---- SPを割込みコンテキストのものに切替え */
                mov     r1, sp                                  /* タスクのSPを保存 */
                ldr     sp, [r0, #CPUCB_ISP]                    /* 割り込み用スタックに切り替え */
                stmfd   sp!, {r1, r3}                           /* タスクのSPと旧imask保存 */
                
            /* ---- 割込み開始処理 */
                bl      _kernel_interrupt_start                 /* 割り込み開始 */
                
            /* ---- 割込みハンドラ実行 */
                mov     r0, #INTNO_IRQ
                bl      _kernel_interrupt_handler
                
            /* ---- 割込み処理の終了設定 */
                ldmfd   sp!, {r1, r3}                           /* 汎用レジスタ復帰 */
                mov     sp, r1                                  /* SPを元のコンテキストのものに戻す */
                ldr     r0, =_KERNEL_CPU_CB                     /* 割り込みネストカウンタのアドレス取得 */
                mov     r1, #0                                  /* 割り込みネストカウンタを0に戻す */
                strb    r1, [r0, #CPUCB_INTCNT]                 /* 割り込みネストカウンタ値設定 */
            
            /* ---- 割込みマスク値復帰処理 */
                strb    r3, [r0, #CPUCB_IMSK]                   /* マスク値復帰 */
            
            /* ---- 割込み終了処理 */
                bl      _kernel_interrupt_end                   /* 割り込み終了処理 */
                
            /* ---- 復帰先割込みマスク設定 */
                ldr     r0, =_KERNEL_CPU_CB
                ldr     r1, [sp, #4]                            /* spsr_irq 値取り出し */
                tst     r1, #I_Bit                              /* 割禁と同時にIRQが入る場合があるのでケア */
                ldreqb  r0, [r0, #CPUCB_IMSK]                   /* この時点でのimsk値取り出し */
                biceq   r1, r1, #(F_Bit | I_Bit)
                orreq   r1, r1, r0
                streq   r1, [sp, #4]                            /* spsr_irq にimsk値反映 */
                
return_int:
            /* ---- 復帰処理 */
                ldmfd   sp!, {r0, r1}                           /* lr_irq, spsr_irq 復帰 */
                msr     cpsr_c, #(Mode_IRQ | F_Bit | I_Bit)     /* IRQモードに移行 */
                mov     lr, r0                                  /* lr_irq 復帰 */
                msr     spsr_csxf, r1                           /* spsr_irq 復帰 */
                msr     cpsr_c, #(Mode_SYS | F_Bit | I_Bit)     /* SYSモードに移行 */

#ifdef _KERNEL_ARM_WITH_VFP     /* VFP利用時 */
                pop     {r1}                                    /* fpexc 復帰 */
                vmsr    fpexc, r1                               /* fpexc 設定 */
                pop     {r1}                                    /* fpscr 復帰 */
                vmsr    fpscr, r1                               /* fpscr 設定 */
                vpop    {d0-d7}                                 /* VFP用レジスタ復帰 */
#endif

                ldmfd   sp!, {r0-r3, r12, lr}                   /* SYSモードの汎用レジスタ復帰 */
                msr     cpsr_c, #(Mode_IRQ | F_Bit | I_Bit)     /* IRQモードに移行 */
                subs    pc, lr, #4                              /* 割込み復帰 */
                
                
            /* ---- 多重割り込み処理 */
multiple_int:
            /* ---- スタックを8バイトアライメントに移動 */
                mov     r1,  sp                                 /* 割り込み発生時のSYSモードのSPに8の倍数の保証されないので */
                bic     sp, sp, #0x7
                stmfd   sp!, {r0, r1}                           /* タスクのSPを割り込み用スタックに保存(8の倍数になるようにr1も) */
                
            /* ---- 割り込みハンドラ実行 */
                mov     r0, #INTNO_IRQ
                bl      _kernel_interrupt_handler

            /* ---- スタックアライメント復帰 */
                ldmfd   sp!, {r0, r1}                           /* 汎用レジスタ復帰 */
                mov     sp, r1
                
            /* ---- 割り込みカウンタ復帰 */
                ldrb    r1, [r0, #CPUCB_INTCNT]
                add     r1, r1, #1
                strb    r1, [r0, #CPUCB_INTCNT]
                b       return_int                              /* 復帰処理 */


                .end


/* end of file */
