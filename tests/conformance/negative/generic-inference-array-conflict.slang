T pick<T>(T first, T second) { return first; }
int run()
{
    int first[2] = {1, 2};
    float second[2] = {3, 4};
    return int(pick(first, second)[0]);
}
