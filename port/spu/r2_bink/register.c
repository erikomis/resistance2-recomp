/* Resistance 2 raw-SPU Bink decoder (igBinkRawSpu) embedded as raw data in
 * EBOOT.elf. EBOOT VA 0x00EB6580, 0x18010 bytes, which igInitRawSpu streams
 * into local store at 0x200 and enters at 0x230. Not an ELF, so it is matched
 * by source EA when that first chunk lands, not by fingerprint. */
#include <stdint.h>

extern void spu_begin_image(int image_id);
extern void spu_overlay_register_region(uint32_t content_ea, uint32_t span, int image_id);
extern void r2_bink_spu_recomp_register(void);

enum { R2_BINK_IMAGE_ID = 910004 };

__attribute__((constructor)) static void r2_bink_register(void)
{
    spu_begin_image(R2_BINK_IMAGE_ID);
    r2_bink_spu_recomp_register();
    spu_overlay_register_region(0x00EB6580u, 0x18010u, R2_BINK_IMAGE_ID);
    spu_begin_image(0);
}
