#!/usr/bin/env python3
"""
Owonero Build Script
Cross-platform build script for Python version
"""

import os
import sys
import platform
import subprocess
import shutil
import argparse
from pathlib import Path

def run_command(cmd, cwd=None, check=True):
    """Run a shell command"""
    try:
        result = subprocess.run(cmd, shell=True, cwd=cwd, capture_output=True, text=True)
        if check and result.returncode != 0:
            print(f"Command failed: {cmd}")
            print(f"Error: {result.stderr}")
            return False
        return True
    except Exception as e:
        print(f"Failed to run command: {e}")
        return False

def get_platform_info():
    """Get platform information"""
    system = platform.system().lower()
    arch = platform.machine().lower()

    # Normalize architecture
    if arch in ['x86_64', 'amd64']:
        arch = 'amd64'
    elif arch in ['i386', 'i686', 'x86']:
        arch = '386'
    elif arch == 'aarch64':
        arch = 'arm64'

    return system, arch

def install_dependencies():
    """Install Python dependencies"""
    print("Installing dependencies...")
    if run_command("pip install -r requirements.txt"):
        print("✓ Dependencies installed")
        return True
    else:
        print("✗ Failed to install dependencies")
        return False

def run_tests():
    """Run tests"""
    print("Running tests...")
    # For now, just check if imports work
    test_script = """
import sys
sys.path.insert(0, 'src')
try:
    import utils
    import blockchain
    import wallet
    import daemon
    import miner
    import wallet_tui
    import web_stats
    import main
    print("All imports successful")
except ImportError as e:
    print(f"Import error: {e}")
    sys.exit(1)
"""
    with open('test_imports.py', 'w') as f:
        f.write(test_script)

    if run_command("python test_imports.py"):
        print("✓ Tests passed")
        os.remove('test_imports.py')
        return True
    else:
        print("✗ Tests failed")
        os.remove('test_imports.py')
        return False

def create_executable():
    """Create executable using PyInstaller or similar"""
    print("Creating executable...")

    system, arch = get_platform_info()
    exe_name = f"owonero-{system}-{arch}"

    if system == "windows":
        exe_name += ".exe"

    # Use PyInstaller if available
    if shutil.which('pyinstaller'):
        cmd = f'pyinstaller --onefile --name {exe_name} src/main.py'
        if run_command(cmd):
            # Move executable to bin directory
            os.makedirs('bin', exist_ok=True)
            if system == "windows":
                src_path = f"dist/{exe_name}"
            else:
                src_path = f"dist/{exe_name}"

            if os.path.exists(src_path):
                dest_path = f"bin/{exe_name}"
                shutil.move(src_path, dest_path)
                print(f"✓ Executable created: {dest_path}")

                # Clean up
                shutil.rmtree('build', ignore_errors=True)
                shutil.rmtree('dist', ignore_errors=True)
                os.remove(f'{exe_name}.spec')

                return True

    # Fallback: create a simple wrapper script
    print("PyInstaller not found, creating wrapper script...")
    os.makedirs('bin', exist_ok=True)

    if system == "windows":
        wrapper = f'''@echo off
python "%~dp0\\..\\src\\main.py" %*
'''
    else:
        wrapper = f'''#!/bin/bash
DIR="$( cd "$( dirname "${{BASH_SOURCE[0]}}" )" &> /dev/null && pwd )"
python3 "$DIR/../src/main.py" "$@"
'''

    wrapper_path = f"bin/{exe_name}"
    with open(wrapper_path, 'w') as f:
        f.write(wrapper)

    if system != "windows":
        os.chmod(wrapper_path, 0o755)

    print(f"✓ Wrapper script created: {wrapper_path}")
    return True

def build_all():
    """Build everything"""
    print("Building Owonero...")
    print("=" * 50)

    success = True

    if not install_dependencies():
        success = False

    if not run_tests():
        success = False

    if not create_executable():
        success = False

    if success:
        print("=" * 50)
        print("✓ Build completed successfully!")
        system, arch = get_platform_info()
        exe_name = f"owonero-{system}-{arch}"
        if platform.system().lower() == "windows":
            exe_name += ".exe"
        print(f"Executable: bin/{exe_name}")
    else:
        print("=" * 50)
        print("✗ Build failed!")
        sys.exit(1)

def clean():
    """Clean build artifacts"""
    print("Cleaning build artifacts...")
    dirs_to_remove = ['build', 'dist', '__pycache__', 'bin']
    files_to_remove = ['*.spec', 'test_imports.py']

    for dir_name in dirs_to_remove:
        if os.path.exists(dir_name):
            shutil.rmtree(dir_name, ignore_errors=True)
            print(f"Removed directory: {dir_name}")

    for pattern in files_to_remove:
        import glob
        for file_path in glob.glob(pattern):
            os.remove(file_path)
            print(f"Removed file: {file_path}")

    print("✓ Clean completed")

def main():
    parser = argparse.ArgumentParser(description='Owonero Build Script')
    parser.add_argument('--clean', action='store_true', help='Clean build artifacts')
    parser.add_argument('--deps', action='store_true', help='Install dependencies only')
    parser.add_argument('--test', action='store_true', help='Run tests only')

    args = parser.parse_args()

    if args.clean:
        clean()
    elif args.deps:
        install_dependencies()
    elif args.test:
        run_tests()
    else:
        build_all()

if __name__ == '__main__':
    main()