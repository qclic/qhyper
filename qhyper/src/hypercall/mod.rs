

numeric_enum_macro::numeric_enum! {
    #[repr(u64)]
    #[derive(Debug, Eq, PartialEq, Copy, Clone)]
    pub enum HyperCallCode {
        HvVirtioInit = 0,
        HvVirtioInjectIrq = 1,
        HvZoneStart = 2,
        HvZoneShutdown = 3,
        HvZoneList = 4,
        HvClearInjectIrq = 20,
        HvIvcInfo = 5,
    }
}

