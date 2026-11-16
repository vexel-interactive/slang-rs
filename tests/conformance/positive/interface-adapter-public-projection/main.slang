module main;
import lib;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    LibraryValue value;
    int result = value.read(1.0);
}
