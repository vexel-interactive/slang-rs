#line 0X10
#if __LINE__ != 0
#error uppercase hex conversion
#endif
#line 0b100
#if __LINE__ != 0
#error binary conversion
#endif
#line 0x10u
#if __LINE__ != 16
#error lowercase hex conversion
#endif
#line 99999999999999999999999999999999
#if __LINE__ != -1
#error saturating conversion
#endif
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
