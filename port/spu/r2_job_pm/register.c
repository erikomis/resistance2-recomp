/* Resistance 2 WWS job policy module embedded as raw data in EBOOT.elf.
 * EBOOT VA 0x00CF9680, 12,544 bytes, loaded into SPU LS at 0xA00.
 * FNV-1a matches spu_workload_fingerprint over those exact bytes. */
#include "spu_workload.h"

extern void spu_begin_image(int image_id);
extern void r2_job_pm_spu_recomp_register(void);
extern void r2_job_pm_spu_func_00000A00(spu_context*);

enum { R2_JOB_PM_IMAGE_ID = 910003 };

__attribute__((constructor)) static void r2_job_pm_register(void)
{
    spu_begin_image(R2_JOB_PM_IMAGE_ID);
    r2_job_pm_spu_recomp_register();
    spu_workload_register_img(0x924CAF0A30D9F98FULL,
                              r2_job_pm_spu_func_00000A00,
                              R2_JOB_PM_IMAGE_ID, "r2_job_pm_CF9680");
    spu_begin_image(0);
}
