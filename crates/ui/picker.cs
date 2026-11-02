using System;
using System.Runtime.InteropServices;

public class Program {
    [ComImport, Guid("42f85136-db7e-439c-85f1-e4075d135fc8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IFileOpenDialog {
        [PreserveSig] int Show(IntPtr parent);
        void SetFileTypes(uint cFileTypes, IntPtr rgFilterSpec);
        void SetFileTypeIndex(uint iFileType);
        void GetFileTypeIndex(out uint piFileType);
        void Advise();
        void Unadvise();
        void SetOptions(uint fos);
        void GetOptions(out uint fos);
        void SetDefaultFolder(IntPtr psi);
        void SetFolder(IntPtr psi);
        void GetFolder(out IntPtr ppsi);
        void GetCurrentSelection(out IntPtr ppsi);
        void SetFileName([MarshalAs(UnmanagedType.LPWStr)] string pszName);
        void GetFileName([MarshalAs(UnmanagedType.LPWStr)] out string pszName);
        void SetTitle([MarshalAs(UnmanagedType.LPWStr)] string pszTitle);
        void SetOkButtonLabel([MarshalAs(UnmanagedType.LPWStr)] string pszText);
        void SetFileNameLabel([MarshalAs(UnmanagedType.LPWStr)] string pszLabel);
        void GetResult(out IShellItem ppsi);
        void AddPlace();
        void SetDefaultExtension([MarshalAs(UnmanagedType.LPWStr)] string pszDefaultExtension);
        void Close(int hr);
        void SetClientGuid();
        void ClearClientData();
        void SetFilter();
    }

    [ComImport, Guid("43826d1e-e718-42ee-bc55-a1e261c37bfe"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellItem {
        void BindToHandler();
        void GetParent();
        void GetDisplayName(uint sigdnName, [MarshalAs(UnmanagedType.LPWStr)] out string ppszName);
        void GetAttributes();
        void Compare();
    }

    [ComImport, Guid("DC1C5A9C-E88A-4dde-A5A1-60F82A20AEF7")]
    private class FileOpenDialogRCW { }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
    public struct COMDLG_FILTERSPEC {
        [MarshalAs(UnmanagedType.LPWStr)] public string pszName;
        [MarshalAs(UnmanagedType.LPWStr)] public string pszSpec;
    }

    [STAThread]
    public static int Main(string[] args) {
        if (args.Length == 0) return 1;
        string mode = args[0].ToLowerInvariant();
        string title = args.Length > 1 ? args[1] : "";

        var dialog = (IFileOpenDialog)new FileOpenDialogRCW();

        if (mode == "folder") {
            dialog.SetOptions(0x00000020 | 0x00000008); // FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM
            if (!string.IsNullOrEmpty(title)) dialog.SetTitle(title);
        } else {
            dialog.SetOptions(0x00000008 | 0x00001000); // FOS_FORCEFILESYSTEM | FOS_FILEMUSTEXIST
            if (!string.IsNullOrEmpty(title)) dialog.SetTitle(title);
            if (mode == "vsix") {
                var specs = new COMDLG_FILTERSPEC[] {
                    new COMDLG_FILTERSPEC { pszName = "VSIX Packages (*.vsix)", pszSpec = "*.vsix" },
                    new COMDLG_FILTERSPEC { pszName = "All Files (*.*)", pszSpec = "*.*" }
                };
                IntPtr ptr = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(COMDLG_FILTERSPEC)) * specs.Length);
                long cur = ptr.ToInt64();
                for (int i = 0; i < specs.Length; i++) {
                    Marshal.StructureToPtr(specs[i], new IntPtr(cur), false);
                    cur += Marshal.SizeOf(typeof(COMDLG_FILTERSPEC));
                }
                dialog.SetFileTypes((uint)specs.Length, ptr);
            }
        }

        int hr = dialog.Show(IntPtr.Zero);
        if (hr == 0) {
            IShellItem item;
            dialog.GetResult(out item);
            string path;
            item.GetDisplayName(0x80058000, out path); // SIGDN_FILESYSPATH
            if (!string.IsNullOrEmpty(path)) {
                Console.Write(path);
                return 0;
            }
        }
        return 0;
    }
}

