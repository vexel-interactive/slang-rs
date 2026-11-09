T pick<T>(T first, T second) { return first; }
struct First { int value; };
struct Second { int value; };
int run()
{
    First first = {1};
    Second second = {2};
    return pick(first, second).value;
}
