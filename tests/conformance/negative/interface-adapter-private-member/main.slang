module main;
import lib;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    LibraryValue value;
    float result = value.privateOnly(1);
}
