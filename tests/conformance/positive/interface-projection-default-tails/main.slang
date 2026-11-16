module main;
import lib;
int kind(int value) { return 1; }
int kind(float value) { return 2; }
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    PublicOne publicOne;
    InternalOne internalOne;
    PublicZero publicZero;
    InternalZero internalZero;
    output[0] = uint(kind(publicOne.read(1.0)));
    output[1] = uint(insidePublicOne());
    output[2] = uint(publicOne.read(1.0));
    output[3] = uint(kind(internalOne.read(1.0)));
    output[4] = uint(insideInternalOne());
    output[5] = uint(internalOne.read(1.0));
    output[6] = uint(kind(publicZero.read()));
    output[7] = uint(insidePublicZero());
    output[8] = uint(publicZero.read());
    output[9] = uint(kind(internalZero.read()));
    output[10] = uint(insideInternalZero());
    output[11] = uint(internalZero.read());
}
