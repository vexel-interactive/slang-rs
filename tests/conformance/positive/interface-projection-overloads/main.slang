module main;
import lib;
int kind(int value) { return 1; }
int kind(float value) { return 2; }
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Internal internalValue;
    Public publicValue;
    ExactPublic exactPublic;
    ExactInternal exactInternal;
    Unrelated unrelated;
    output[0] = uint(kind(internalValue.read(1.0)));
    output[1] = uint(internalValue.read(1.9));
    output[2] = uint(kind(publicValue.read(1.0)));
    output[3] = uint(kind(publicValue.read(1)));
    output[4] = uint(publicValue.read(1.9));
    output[5] = uint(publicValue.read(2));
    output[6] = uint(kind(exactPublic.read(1.0)));
    output[7] = uint(kind(exactInternal.read(1.0)));
    output[8] = uint(insideExact());
    output[9] = uint(insideAdapted());
    output[10] = uint(unrelated.read(1.9));
    output[11] = uint(unrelated.read(Other(3)));
    ExactWitness exactWitness;
    output[12] = uint(exactWitness.read(1.0));
}
