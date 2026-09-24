namespace StatusShare.WindowsApp;

public partial class MainWindow : global::Wpf.Ui.Controls.FluentWindow
{
    public MainWindow()
    {
        InitializeComponent();
        var viewModel = new ViewModels.MainWindowViewModel();
        DataContext = viewModel;
        Closing += (_, _) => viewModel.FlushAutosave();
    }
}

