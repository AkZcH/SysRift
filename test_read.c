#include <unistd.h>
#include <fcntl.h>

int main() {
    int fd = open("input.txt", O_RDONLY);
    char buf[64] = {0};
    int n = read(fd, buf, sizeof(buf));
    write(1, buf, n);
    write(1, "\n", 1);
    close(fd);
    return 0;
}
