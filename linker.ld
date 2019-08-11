ENTRY(_start)

base = 0xFFFFFFFF80000000;
/*offset = 0x100000;*/
offset = 0;

SECTIONS {
	. = base + offset;

	.text : ALIGN(0x1000) {
		*(*.text)
        *(*.text.*)
	}

	.data : ALIGN(0x1000) {
		*(*.data)
        *(*.data.*)
	}

	.got : ALIGN(0x1000) {
		*(*.got)
        *(*.got.*)
	}

	.rodata : ALIGN(0x1000) {
		*(*.rodata)
        *(*.rodata.*)
	}

	.bss : ALIGN(0x1000) {
		*(*.bss)
        *(*.bss.*)
	}
}
