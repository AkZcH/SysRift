#include <unistd.h>
#include <sys/wait.h>

int main() {
    pid_t pid = fork();
    if (pid == 0) {
        // child
        write(1, "child\n", 6);
    } else {
        // parent
        wait(NULL);
        write(1, "parent\n", 7);
    }
    return 0;
}
