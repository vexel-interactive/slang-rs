// Character values transcribed from the pinned frontend literal corpus.
RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = ' ';
    output[1] = 'X';
    output[2] = ']';
    output[3] = '~';
    output[4] = 'ä';
    output[5] = '€';
    output[6] = '☺';
    output[7] = '􏿽';
    output[8] = '\'';
    output[9] = '\"';
    output[10] = '\\';
    output[11] = '\?';
    output[12] = '\a';
    output[13] = '\b';
    output[14] = '\f';
    output[15] = '\n';
    output[16] = '\r';
    output[17] = '\t';
    output[18] = '\v';
    output[19] = '\0';
    output[20] = '\10';
    output[21] = '\25';
    output[22] = '\77';
    output[23] = '\123';
    output[24] = '\377';
    output[25] = '\x0';
    output[26] = '\x12';
    output[27] = '\x7f';
    output[28] = '\x7F';
    output[29] = '\xFEDC';
    output[30] = '\xFEDC1234';
    output[31] = '\xfFfFfFfF';
    output[32] = '\x{0}';
    output[33] = '\x{12}';
    output[34] = '\x{7f}';
    output[35] = '\x{7F}';
    output[36] = '\x{FEDC}';
    output[37] = '\x{FEDC1234}';
    output[38] = '\x{fFfFfFfF}';
    output[39] = '\u0000';
    output[40] = '\u0012';
    output[41] = '\u1234';
    output[42] = '\ufFfF';
    output[43] = '\u{0}';
    output[44] = '\u{012}';
    output[45] = '\u{1234}';
    output[46] = '\u{fFfF}';
    output[47] = '\u{10ffff}';
    output[48] = '\u{ffffffff}';
    output[49] = '\u{00fefefefe}';
    output[50] = '\U00000012';
    output[51] = '\U00001234';
    output[52] = '\U0000fFfF';
    output[53] = '\U0010ffff';
    output[54] = '\Uffffffff';
}
